//! Acesso ao headset MCHOSE V9 PRO pelo `/dev/hidraw`, com hotplug.
//!
//! Descobre o dispositivo, mantem um fluxo de eventos vivo enquanto o dongle
//! existir, e sobrevive a ele sumir e voltar. Nao conhece `iced` nem
//! `libcosmic`: quem embrulha o `Stream` numa `Subscription` e o applet.
//!
//! Ver `README.md` e `crates/mchose-device/AGENTS.md`.

// `deny`, nao `forbid`: os dois ioctls de feature do hidraw nao tem involucro
// seguro na std, entao o modulo de transporte real precisa de `unsafe`. `deny`
// permite abrir a excecao num lugar so e deixa o resto do crate fechado;
// `forbid` nao poderia ser aberto nem com justificativa.
#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]
pub(crate) mod discovery;
pub(crate) mod machine;

// A superficie publica do crate e o evento e, na task 6, o `Stream`. O trait de
// transporte e a sessao ficam fechados: exportar qualquer um dos dois daria ao
// consumidor escrita de bytes crus no canal vendor, que e o que a API fechada do
// `mchose-protocol` existe para impedir.
pub use machine::DeviceEvent;
pub(crate) mod hotplug;
pub(crate) mod transport;

/// Alca para pedir uma leitura nova fora do ritmo do push.
///
/// `Clone + Send`: o applet guarda uma copia e pede quando abre o popover.
#[derive(Debug, Clone)]
pub struct Demand(tokio::sync::mpsc::Sender<()>);

impl Demand {
    /// Pede uma leitura. Com o dongle ausente e no-op — nunca erro, e nunca
    /// espera.
    ///
    /// Sincrona de proposito: sem sessao ninguem drena o canal, e um `send`
    /// assincrono penduraria o chamador assim que a fila enchesse. Abrir o
    /// popover nao pode travar o painel.
    pub fn refresh(&self) {
        let _ = self.0.try_send(());
    }
}

/// Abre o fluxo de eventos do dispositivo.
///
/// O `Stream` nao termina quando o dongle some: quem reata e a supervisao, e fim
/// de stream seria fim de applet.
///
/// Nao exige runtime tokio ambiente. A supervisao roda numa thread propria com
/// runtime `current_thread` — nao por gosto, mas porque o socket do monitor do
/// kernel guarda ponteiros crus e **nao e `Send`**: nao atravessa thread e
/// portanto nao entra numa task compartilhada. Confinar o socket numa thread so
/// resolve, e o canal de eventos, esse sim `Send`, atravessa de volta.
pub fn events() -> std::io::Result<(
    impl futures::Stream<Item = DeviceEvent> + Send + 'static,
    Demand,
)> {
    let (tx_eventos, rx_eventos) = tokio::sync::mpsc::channel(50);
    let (tx_pedidos, rx_pedidos) = tokio::sync::mpsc::channel(4);

    let emissor = tx_eventos.clone();
    std::thread::Builder::new()
        .name("mchose-device".into())
        .spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
            else {
                return;
            };
            runtime.block_on(async move {
                let Ok(fonte) = hotplug::KernelSource::new() else {
                    return;
                };
                hotplug::supervise(fonte, rx_pedidos, move |e| {
                    let _ = emissor.try_send(e);
                })
                .await;
            });
        })?;

    let fluxo = futures::stream::unfold(rx_eventos, |mut rx| async move {
        rx.recv().await.map(|e| (e, rx))
    });
    Ok((fluxo, Demand(tx_pedidos)))
}
