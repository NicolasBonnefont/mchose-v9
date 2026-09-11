//! Imprime o fluxo de eventos do dispositivo.
//!
//! E o que prova a cadeia sem depender do compositor. Nao imprime byte cru: o
//! `Rejected` vira contagem e o `PermissionDenied` nao mostra o caminho do
//! device — log de terminal acaba colado em issue de repositorio publico.
//!
//! ```text
//! cargo run -p mchose-device --example eventos
//! ```

use futures::StreamExt;
use mchose_device::{DeviceEvent, events};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (fluxo, _demand) = match events() {
        Ok(par) => par,
        Err(e) => {
            eprintln!("nao foi possivel abrir o fluxo: {e}");
            return;
        }
    };
    futures::pin_mut!(fluxo);

    let mut rejeitados: usize = 0;
    while let Some(evento) = fluxo.next().await {
        match evento {
            DeviceEvent::Connected { dongle, headset } => {
                let d = dongle.map_or("desconhecida".into(), |v| v.vendor_string());
                let h = headset.map_or("desconhecida".into(), |v| v.vendor_string());
                println!("conectado — firmware do dongle {d}, do fone {h}");
            }
            DeviceEvent::Battery(b) => println!("bateria {}% — {:?}", b.percent, b.state),
            // As frases sao do applet — o pack deste crate diz que aqui so se
            // publica o estado. Duas copias divergiriam no primeiro card que
            // mexesse na tabela.
            // Arm proprio, e nao o `outro` abaixo: o `Debug` de
            // `PermissionDenied` carrega o caminho do device, e isto aqui vai
            // colado em issue de repositorio publico.
            DeviceEvent::PermissionDenied { .. } => {
                println!("sem acesso ao dispositivo — instale a regra udev");
            }
            DeviceEvent::Rejected(_) => {
                rejeitados = rejeitados.saturating_add(1);
                println!("pacote rejeitado (total: {rejeitados})");
            }
            outro => println!("{outro:?}"),
        }
    }
    println!("o fluxo terminou");
}
