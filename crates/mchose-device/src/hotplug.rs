//! Supervisao: mantem o fluxo vivo enquanto o dongle vai e volta.
//!
//! O laco nunca termina por conta propria. Fim de stream seria fim de applet —
//! e o padrao dos applets do COSMIC e exatamente este: estado, espera, reentra.

use std::path::Path;

use crate::machine::{DeviceEvent, run_session};
use crate::transport::Transport;
use crate::transport::hidraw::HidrawTransport;

/// Onde o sysfs dos hidraw vive.
const RAIZ_SYSFS: &str = "/sys/class/hidraw";

/// De onde vem o transporte de cada sessao.
///
/// Em producao, monitor do kernel mais enumeracao; em teste, uma fila de
/// transportes falsos. A supervisao nao sabe a diferenca — e e de proposito que
/// o fake encena o **transporte**, nunca o sysfs.
pub(crate) trait Source {
    /// O transporte que esta fonte entrega.
    type Out: Transport;
    /// Espera o proximo dispositivo. `None` encerra a supervisao.
    fn next(&mut self) -> impl Future<Output = Option<Self::Out>>;
}

/// Conduz sessoes sucessivas, uma por vida do dongle.
pub(crate) async fn supervise<S, F>(
    mut source: S,
    mut demand: tokio::sync::mpsc::Receiver<()>,
    mut emit: F,
) where
    S: Source,
    F: FnMut(DeviceEvent),
{
    while let Some(transporte) = source.next().await {
        // A alca de consulta atravessa as sessoes: o consumidor nao reassina
        // nada quando o dongle volta.
        demand = run_session(transporte, demand, &mut emit).await;
    }
}

/// A fonte real: monitor do kernel mais enumeracao no sysfs.
pub(crate) struct KernelSource {
    monitor: AsyncMonitor,
    eventos: tokio::sync::mpsc::Sender<DeviceEvent>,
}

impl KernelSource {
    /// O monitor nasce **antes** da primeira enumeracao: um dispositivo ja
    /// plugado quando o applet abre precisa disparar a mesma consulta que uma
    /// chegada dispararia, senao o painel fica esperando um push que pode nao
    /// vir.
    pub(crate) fn new(eventos: tokio::sync::mpsc::Sender<DeviceEvent>) -> std::io::Result<Self> {
        let monitor = AsyncMonitor::new()?;
        Ok(Self { monitor, eventos })
    }
}

impl Source for KernelSource {
    type Out = HidrawTransport;

    async fn next(&mut self) -> Option<HidrawTransport> {
        loop {
            if let Some(caminho) = crate::discovery::find_device(Path::new(RAIZ_SYSFS)) {
                match HidrawTransport::open(&caminho) {
                    Ok(t) => return Some(t),
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        let _ = self
                            .eventos
                            .try_send(DeviceEvent::PermissionDenied { caminho });
                    }
                    Err(_) => {}
                }
            }
            self.monitor.proxima_mudanca().await?;
        }
    }
}

/// Monitor de chegada e saida, embrulhado para espera assincrona.
pub(crate) struct AsyncMonitor {
    fd: tokio::io::unix::AsyncFd<udev::MonitorSocket>,
}

impl AsyncMonitor {
    pub(crate) fn new() -> std::io::Result<Self> {
        let socket = udev::MonitorBuilder::new()?
            .match_subsystem("hidraw")?
            .listen()?;
        Ok(Self {
            fd: tokio::io::unix::AsyncFd::new(socket)?,
        })
    }

    /// Bloqueia ate o kernel anunciar alguma mudanca em `hidraw`.
    ///
    /// O evento apenas sinaliza: a identificacao vem sempre do sysfs, nunca dos
    /// atributos do evento — um dispositivo virtual nao tem pai USB e nao
    /// carrega `idVendor`.
    async fn proxima_mudanca(&mut self) -> Option<()> {
        let mut pronto = self.fd.readable_mut().await.ok()?;
        let houve = pronto.get_inner_mut().iter().next().is_some();
        pronto.clear_ready();
        houve.then_some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::FakeTransport;

    const BATERIA: &[u8] = &[0x55, 0x65, 0x46, 0x02];
    const FW: &[u8] = &[0xAA, 0x01, 0x00, 0x00, 0x01, 0x02];

    fn fake() -> FakeTransport {
        FakeTransport::new()
            .com_feature(0xAA, FW.to_vec())
            .com_leitura(BATERIA.to_vec())
            .que_some_depois_das_leituras()
    }

    /// Fonte de teste: uma fila de vidas do dongle.
    struct Fila(std::vec::IntoIter<FakeTransport>);

    impl Source for Fila {
        type Out = FakeTransport;
        async fn next(&mut self) -> Option<FakeTransport> {
            self.0.next()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn dongle_que_some_e_volta_produz_leitura_nas_duas_vidas() {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        drop(tx);
        let mut eventos = Vec::new();
        supervise(Fila(vec![fake(), fake()].into_iter()), rx, |e| {
            eventos.push(e)
        })
        .await;

        let leituras = eventos
            .iter()
            .filter(|e| matches!(e, DeviceEvent::Battery(_)))
            .count();
        assert_eq!(leituras, 2, "uma leitura por vida do dongle");
        let quedas = eventos
            .iter()
            .filter(|e| matches!(e, DeviceEvent::Disconnected))
            .count();
        assert_eq!(quedas, 2, "cada vida termina com Disconnected");
    }
}
