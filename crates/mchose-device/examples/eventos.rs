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
            DeviceEvent::NoResponse => {
                println!("fone nao respondeu — pode estar desligado ou carregando");
            }
            DeviceEvent::Disconnected => println!("dongle removido"),
            DeviceEvent::NoDevice => println!("dongle nao encontrado"),
            DeviceEvent::PermissionDenied { .. } => {
                println!("sem acesso ao dispositivo — instale a regra 72-mchose-v9.rules");
            }
            DeviceEvent::Rejected(_) => {
                rejeitados = rejeitados.saturating_add(1);
                println!("pacote rejeitado (total: {rejeitados})");
            }
            _ => println!("evento novo, ainda sem tratamento"),
        }
    }
    println!("o fluxo terminou");
}
