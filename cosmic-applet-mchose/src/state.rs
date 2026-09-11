//! O que mostrar, dado o que o dispositivo disse.
//!
//! Este modulo **nao conhece libcosmic**. Ele recebe `DeviceEvent` e devolve
//! frase, percentual e firmware; a camada de UI so traduz isso em widget. E essa
//! fronteira que torna o applet testavel sem painel — sem ela, as sete frases
//! cairiam dentro de um `update()` e nada seria verificavel.

use mchose_device::{ChargeState, DeviceEvent, FirmwareVersion};

/// O que a interface precisa saber.
#[derive(Debug, Default, Clone)]
pub struct AppletState {
    percent: Option<u8>,
    status: Option<String>,
    dongle: Option<FirmwareVersion>,
    headset: Option<FirmwareVersion>,
}

/// Como um estado de carga se le em portugues.
fn carga_por_extenso(state: ChargeState) -> &'static str {
    match state {
        ChargeState::Discharging => "descarregando",
        ChargeState::Charging => "carregando",
        ChargeState::Full => "bateria cheia",
        ChargeState::Asleep => "dormindo",
        // O byte cru nao entra na frase: ele e dado de dispositivo, e a UI nao
        // publica isso. Quem quiser mapea-lo abre um card.
        _ => "estado desconhecido",
    }
}

impl AppletState {
    /// Aplica um evento do dispositivo.
    pub fn apply(&mut self, evento: DeviceEvent) {
        match evento {
            DeviceEvent::Connected { dongle, headset } => {
                // `None` significa "o fone nao respondeu agora", nao "a versao
                // deixou de existir": versao conhecida nao e apagada.
                if dongle.is_some() {
                    self.dongle = dongle;
                }
                if headset.is_some() {
                    self.headset = headset;
                }
                self.status = Some("dongle conectado".to_owned());
            }
            DeviceEvent::Battery(leitura) => {
                self.percent = Some(leitura.percent);
                self.status = Some(carga_por_extenso(leitura.state).to_owned());
            }
            // O fone plugado para carregar deixa o dongle vivo e o radio mudo:
            // e aqui que "pode estar carregando" cabe, nao no `Disconnected`.
            DeviceEvent::NoResponse => {
                self.status =
                    Some("fone não respondeu — pode estar desligado ou carregando".to_owned());
            }
            // O transporte morreu, ou seja o dongle saiu do USB. E transitorio:
            // vem `NoDevice` ou um `Connected` novo atras.
            DeviceEvent::Disconnected => self.status = Some("dongle removido".to_owned()),
            DeviceEvent::NoDevice => self.status = Some("dongle não encontrado".to_owned()),
            // O caminho do device fica de fora da frase de proposito.
            DeviceEvent::PermissionDenied { .. } => {
                self.status = Some(
                    "sem acesso ao dispositivo — instale a regra 72-mchose-v9.rules".to_owned(),
                );
            }
            // Pacote do nosso canal que nao decodificou. Nao muda a UI, e os
            // bytes nao chegam a lugar nenhum.
            DeviceEvent::Rejected(_) => {}
            // `DeviceEvent` e `#[non_exhaustive]`: variante nova nao quebra o
            // build, preserva o percentual e nao mexe na frase. Inalcancavel a
            // partir daqui, entao sem teste possivel.
            _ => {}
        }
    }

    /// Registra que o fluxo de eventos caiu e que haverá nova tentativa.
    pub fn stream_failed(&mut self, motivo: &str) {
        self.status = Some(format!(
            "sem contato com o dispositivo, reconectando ({motivo})"
        ));
    }

    /// O percentual para o painel. `None` quando nunca houve leitura.
    pub const fn panel_percent(&self) -> Option<u8> {
        self.percent
    }

    /// A frase de estado, para o popover.
    pub fn status(&self) -> &str {
        self.status.as_deref().unwrap_or("procurando o dongle")
    }

    /// Versao de firmware do dongle, pronta para exibir.
    pub fn dongle_firmware(&self) -> String {
        Self::versao(self.dongle.as_ref())
    }

    /// Versao de firmware do fone, pronta para exibir.
    pub fn headset_firmware(&self) -> String {
        Self::versao(self.headset.as_ref())
    }

    fn versao(v: Option<&FirmwareVersion>) -> String {
        v.map_or_else(|| "desconhecida".to_owned(), FirmwareVersion::vendor_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mchose_device::{BatteryReading, ChargeState, DeviceEvent, FirmwareVersion};

    fn fw(bytes: [u8; 4]) -> FirmwareVersion {
        FirmwareVersion::new(bytes)
    }

    fn leitura(percent: u8, state: ChargeState) -> DeviceEvent {
        DeviceEvent::Battery(BatteryReading::new(percent, state))
    }

    #[test]
    fn sem_evento_nenhum_nao_inventa_percentual() {
        let s = AppletState::default();
        assert_eq!(s.panel_percent(), None);
    }

    #[test]
    fn cada_evento_tem_a_frase_declarada_na_tabela() {
        let casos: Vec<(DeviceEvent, &str)> = vec![
            (DeviceEvent::NoDevice, "dongle não encontrado"),
            (
                DeviceEvent::NoResponse,
                "fone não respondeu — pode estar desligado ou carregando",
            ),
            (DeviceEvent::Disconnected, "dongle removido"),
        ];
        for (evento, esperado) in casos {
            let mut s = AppletState::default();
            s.apply(evento);
            assert_eq!(s.status(), esperado);
        }
    }

    #[test]
    fn desconectado_nao_diz_carregando_e_sem_dongle_nao_diz_desconectado() {
        let mut a = AppletState::default();
        a.apply(DeviceEvent::Disconnected);
        assert!(!a.status().contains("carregando"));
        let mut b = AppletState::default();
        b.apply(DeviceEvent::NoDevice);
        assert!(!b.status().contains("removido"));
    }

    #[test]
    fn sem_permissao_nomeia_a_regra_e_nao_mostra_o_caminho() {
        let mut s = AppletState::default();
        s.apply(DeviceEvent::PermissionDenied {
            path: "/dev/hidraw5".into(),
        });
        assert!(s.status().contains("72-mchose-v9.rules"));
        assert!(!s.status().contains("/dev/hidraw"));
    }

    #[test]
    fn o_ultimo_percentual_sobrevive_a_silencio_e_a_remocao() {
        let mut s = AppletState::default();
        s.apply(leitura(70, ChargeState::Discharging));
        s.apply(DeviceEvent::NoResponse);
        assert_eq!(s.panel_percent(), Some(70), "silencio nao apaga o numero");
        s.apply(DeviceEvent::Disconnected);
        assert_eq!(s.panel_percent(), Some(70), "remocao nao apaga o numero");
    }

    #[test]
    fn conexao_publica_o_firmware_e_none_nao_apaga_o_que_ja_sabemos() {
        let mut s = AppletState::default();
        s.apply(DeviceEvent::Connected {
            dongle: Some(fw([0, 0, 1, 2])),
            headset: Some(fw([0, 0, 3, 6])),
        });
        assert_eq!(s.dongle_firmware(), "0012");
        assert_eq!(s.headset_firmware(), "0036");

        s.apply(DeviceEvent::Connected {
            dongle: None,
            headset: None,
        });
        assert_eq!(
            s.dongle_firmware(),
            "0012",
            "None nao apaga versao conhecida"
        );
        assert_eq!(s.headset_firmware(), "0036");
    }

    #[test]
    fn firmware_desconhecido_tem_palavra_propria() {
        let s = AppletState::default();
        assert_eq!(s.dongle_firmware(), "desconhecida");
    }

    #[test]
    fn pacote_rejeitado_nao_muda_a_ui_nem_vaza_byte() {
        let mut s = AppletState::default();
        s.apply(leitura(70, ChargeState::Discharging));
        let antes = s.status().to_owned();
        s.apply(DeviceEvent::Rejected(vec![0x55, 0x65, 0xC8, 0x02]));
        assert_eq!(s.status(), antes, "rejeitado nao muda a frase");
        assert_eq!(s.panel_percent(), Some(70));
        // Nenhuma superficie do estado pode conter os bytes.
        let superficies = [
            s.status().to_owned(),
            s.dongle_firmware(),
            s.headset_firmware(),
        ];
        for texto in superficies {
            assert!(!texto.contains("85") && !texto.contains("0x55") && !texto.contains("200"));
        }
    }

    #[test]
    fn falha_de_fluxo_vira_estado_visivel() {
        let mut s = AppletState::default();
        s.stream_failed("qualquer coisa");
        assert!(s.status().contains("reconectando"));
    }

    #[test]
    fn o_estado_de_carga_aparece_por_extenso() {
        let mut s = AppletState::default();
        s.apply(leitura(100, ChargeState::Full));
        assert!(s.status().contains("cheia"));
        s.apply(leitura(70, ChargeState::Discharging));
        assert!(s.status().contains("descarregando"));
        s.apply(leitura(70, ChargeState::Asleep));
        assert!(s.status().contains("dormindo"));
    }
}
