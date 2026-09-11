//! Maquina de estados da sessao com o dispositivo.
//!
//! Uma sessao vive enquanto o transporte viver. Quem reata depois que o dongle
//! some e o laco de hotplug: aqui a sessao apenas termina com `Disconnected`.

use crate::transport::Transport;
use mchose_protocol::battery::{BatteryReading, decode_battery};
use mchose_protocol::firmware::{FirmwareVersion, decode_firmware};
use mchose_protocol::request::{
    BATTERY_RESPONSE_TIMEOUT, FEATURE_ROUNDTRIP_WAIT, FirmwareTarget, REPORT_FIRMWARE, REPORT_LEN,
    battery_request, firmware_request,
};

/// O que a sessao publica. Owned de proposito: `NoReading` empresta o buffer de
/// leitura, e o card #3 converte estes eventos em `Message` do iced, que exige
/// `'static`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceEvent {
    /// Dongle encontrado. As versoes sao `None` quando o fone nao respondeu.
    Connected {
        /// Versao do receptor USB.
        dongle: Option<FirmwareVersion>,
        /// Versao do fone.
        headset: Option<FirmwareVersion>,
    },
    /// Leitura valida.
    Battery(BatteryReading),
    /// Consulta feita, nada dentro de [`BATTERY_RESPONSE_TIMEOUT`].
    NoResponse,
    /// Pacote do nosso canal que nao decodificou, com os bytes que o protocolo
    /// delimitou. E o que mantem o protocolo mapeavel.
    Rejected(Vec<u8>),
    /// O transporte morreu. Quem reata e o laco de hotplug.
    Disconnected,
    /// Nenhum `hidraw` casa. Emitido uma vez por ausencia, para o applet poder
    /// distinguir "sem dongle" de "ainda carregando".
    NoDevice,
    /// Abrir o dispositivo devolveu `EACCES`.
    ///
    /// **Nao e "sem dongle".** Significa que `install/72-mchose-v9.rules` nao
    /// foi instalada — verificado em 11/09/2026: sem a regra, `/dev/hidraw5`
    /// fica `crw------- root root`. Quem mostra a mensagem precisa poder dizer
    /// *instale a regra*, e nao *fone desligado*.
    PermissionDenied {
        /// O device node que nao abriu.
        path: std::path::PathBuf,
    },
}

/// Le uma versao de firmware. Devolve `None` se o dispositivo nao responder —
/// firmware ausente nao derruba a sessao.
async fn feature_version<T: Transport>(
    transport: &mut T,
    target: FirmwareTarget,
) -> Option<FirmwareVersion> {
    transport
        .set_feature(&firmware_request(target))
        .await
        .ok()?;
    // Ler antes disso devolve zeros. A espera e assincrona: dormir bloqueando
    // travaria o painel do applet.
    tokio::time::sleep(FEATURE_ROUNDTRIP_WAIT).await;
    let mut buf = [0u8; REPORT_LEN];
    let n = transport
        .get_feature(REPORT_FIRMWARE, &mut buf)
        .await
        .ok()?;
    decode_firmware(buf.get(..n)?).ok()
}

/// O que a espera do laco produziu.
enum Step {
    Read(usize),
    Deadline,
    Request,
    Died,
}

/// Conduz uma sessao do inicio ao fim, publicando eventos por `emit`.
///
/// Retorna quando o transporte morre ou quando o consumidor larga a alca de
/// consulta.
pub(crate) async fn run_session<T, F>(
    mut transport: T,
    demand: &mut tokio::sync::mpsc::Receiver<()>,
    mut emit: F,
) where
    T: Transport,
    F: FnMut(DeviceEvent),
{
    let dongle = feature_version(&mut transport, FirmwareTarget::Dongle).await;
    let headset = feature_version(&mut transport, FirmwareTarget::Headset).await;
    emit(DeviceEvent::Connected { dongle, headset });

    if transport.write(&battery_request()).await.is_err() {
        emit(DeviceEvent::Disconnected);
        return;
    }
    let mut awaiting = true;
    // Um consumidor que so escuta — um binario de terminal, por exemplo — nao
    // segura alca de consulta. O canal fechar significa "nunca havera pedido",
    // nao "acabou a sessao": tratar como fim faria esse consumidor nunca
    // receber evento nenhum.
    let mut has_requester = true;

    let mut buf = [0u8; REPORT_LEN];
    loop {
        // O emprestimo de `transport` e de `buf` termina no fim deste bloco,
        // liberando os dois para a acao logo abaixo. Abandonar a leitura no
        // meio e seguro: nada foi consumido do descritor.
        let step = {
            let leitura = transport.read(&mut buf);
            tokio::pin!(leitura);
            // Prazo ausente e um futuro que nunca resolve: assim o `select` tem
            // sempre a mesma forma, em vez de duas copias da mesma maquina.
            let prazo = async {
                if awaiting {
                    tokio::time::sleep(BATTERY_RESPONSE_TIMEOUT).await;
                } else {
                    std::future::pending::<()>().await;
                }
            };
            tokio::pin!(prazo);
            tokio::select! {
                // `biased` torna a ordem deterministica: leitura pendente tem
                // prioridade sobre prazo e sobre pedido novo.
                biased;
                r = &mut leitura => match r {
                    Ok(n) => Step::Read(n),
                    Err(_) => Step::Died,
                },
                () = &mut prazo => Step::Deadline,
                p = demand.recv(), if has_requester => match p {
                    Some(()) => Step::Request,
                    None => { has_requester = false; continue }
                },
            }
        };

        match step {
            Step::Died => {
                emit(DeviceEvent::Disconnected);
                return;
            }
            Step::Deadline => {
                emit(DeviceEvent::NoResponse);
                awaiting = false;
            }
            Step::Request => {
                if transport.write(&battery_request()).await.is_err() {
                    emit(DeviceEvent::Disconnected);
                    return;
                }
                awaiting = true;
            }
            Step::Read(n) => {
                awaiting = false;
                let Some(pacote) = buf.get(..n) else { continue };
                match decode_battery(pacote) {
                    Ok(leitura) => emit(DeviceEvent::Battery(leitura)),
                    // `rejected` so vem preenchido quando o pacote era do nosso
                    // canal. Tecla de midia e telefonia chegam no mesmo
                    // descritor e nao sao anomalia.
                    Err(falha) => {
                        if let Some(bytes) = falha.rejected {
                            emit(DeviceEvent::Rejected(bytes.to_vec()));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::FakeTransport;
    use mchose_protocol::battery::ChargeState;

    const BATERIA: &[u8] = &[0x55, 0x65, 0x46, 0x02];
    const FW_DONGLE: &[u8] = &[0xAA, 0x01, 0x00, 0x00, 0x01, 0x02];
    const TECLA_MIDIA: &[u8] = &[0x06, 0x01, 0x00];

    /// Roda a sessao ate ela terminar, colecionando os eventos.
    async fn colher(t: FakeTransport) -> Vec<DeviceEvent> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        drop(tx);
        let mut eventos = Vec::new();
        run_session(t, &mut rx, |e| eventos.push(e)).await;
        eventos
    }

    #[tokio::test(start_paused = true)]
    async fn pacote_real_vira_leitura() {
        let t = FakeTransport::new()
            .com_feature(0xAA, FW_DONGLE.to_vec())
            .com_leitura(BATERIA.to_vec())
            .que_some_depois_das_leituras();
        let ev = colher(t).await;
        assert!(ev.iter().any(|e| matches!(
            e,
            DeviceEvent::Battery(b) if b.percent == 70 && b.state == ChargeState::Discharging
        )));
    }

    #[tokio::test(start_paused = true)]
    async fn conexao_publica_o_firmware() {
        let t = FakeTransport::new()
            .com_feature(0xAA, FW_DONGLE.to_vec())
            .que_some_depois_das_leituras();
        let ev = colher(t).await;
        assert!(
            ev.iter()
                .any(|e| matches!(e, DeviceEvent::Connected { .. }))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn sem_resposta_no_prazo_vira_no_response() {
        // Sem leitura enfileirada: a consulta de conexao nao e respondida, e o
        // fone fica em silencio para sempre. A alca segue viva, entao quem
        // corta a sessao e o teste — nao o fim do canal.
        let t = FakeTransport::new().com_feature(0xAA, FW_DONGLE.to_vec());
        let (_tx, mut rx) = tokio::sync::mpsc::channel(4);
        let eventos = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let colecionador = std::sync::Arc::clone(&eventos);
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            run_session(t, &mut rx, move |e| colecionador.lock().unwrap().push(e)),
        )
        .await;
        let vistos = eventos.lock().unwrap().clone();
        assert!(
            vistos.contains(&DeviceEvent::NoResponse),
            "vistos: {vistos:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn trafego_alheio_nao_produz_evento() {
        let t = FakeTransport::new()
            .com_feature(0xAA, FW_DONGLE.to_vec())
            .com_leitura(TECLA_MIDIA.to_vec())
            .com_leitura(BATERIA.to_vec())
            .que_some_depois_das_leituras();
        let ev = colher(t).await;
        let leituras = ev
            .iter()
            .filter(|e| matches!(e, DeviceEvent::Battery(_) | DeviceEvent::Rejected(_)))
            .count();
        assert_eq!(leituras, 1, "a tecla de midia nao pode virar evento");
    }

    #[tokio::test(start_paused = true)]
    async fn pacote_invalido_do_nosso_canal_carrega_os_bytes() {
        let ruim = vec![0x55, 0x65, 200, 0x02];
        let t = FakeTransport::new()
            .com_feature(0xAA, FW_DONGLE.to_vec())
            .com_leitura(ruim.clone())
            .que_some_depois_das_leituras();
        let ev = colher(t).await;
        assert!(ev.contains(&DeviceEvent::Rejected(ruim)));
    }

    #[tokio::test(start_paused = true)]
    async fn transporte_que_some_encerra_a_sessao_com_disconnected() {
        let t = FakeTransport::new().que_some();
        let ev = colher(t).await;
        assert_eq!(ev.last(), Some(&DeviceEvent::Disconnected));
    }

    #[tokio::test(start_paused = true)]
    async fn consulta_sob_demanda_produz_leitura_nova() {
        let t = FakeTransport::new()
            .com_feature(0xAA, FW_DONGLE.to_vec())
            .com_leitura(BATERIA.to_vec())
            .com_leitura(BATERIA.to_vec())
            .que_some_depois_das_leituras();
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        tx.send(()).await.expect("pedido");
        drop(tx);
        let mut eventos = Vec::new();
        run_session(t, &mut rx, |e| eventos.push(e)).await;
        let leituras = eventos
            .iter()
            .filter(|e| matches!(e, DeviceEvent::Battery(_)))
            .count();
        assert_eq!(leituras, 2, "conexao mais a consulta sob demanda");
    }

    async fn escritas_da_sessao(t: FakeTransport) -> Vec<Vec<u8>> {
        let reg = t.registro();
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        drop(tx);
        run_session(t, &mut rx, |_| {}).await;
        reg.lock().unwrap().clone()
    }

    #[tokio::test(start_paused = true)]
    async fn em_estado_estacionario_nenhuma_consulta_e_emitida() {
        let t = FakeTransport::new()
            .com_feature(0xAA, FW_DONGLE.to_vec())
            .com_leitura(BATERIA.to_vec())
            .que_some_depois_das_leituras();
        let escritas = escritas_da_sessao(t).await;
        // Duas de firmware (set dongle, set fone) e uma de bateria na conexao.
        let consultas_bateria = escritas.iter().filter(|w| w.first() == Some(&0x55)).count();
        assert_eq!(consultas_bateria, 1, "so a consulta de conexao");
    }
}
