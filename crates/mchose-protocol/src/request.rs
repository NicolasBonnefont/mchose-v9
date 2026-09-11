//! Construcao dos pacotes enviados ao dispositivo.
//!
//! A API aqui e **fechada de proposito**: sao exatamente dois construtores, e
//! nenhuma funcao aceita report id ou payload do chamador. O descritor tem
//! ainda os report ids `0xED` e `0x41`, de conteudo desconhecido; expor um
//! construtor generico daria ao resto do projeto escrita arbitraria no canal
//! vendor do firmware, que e caminho conhecido para brickar dispositivo.

use core::time::Duration;

/// Report id do canal de bateria (usage page `0xFF90`), entrada e saida.
pub const REPORT_BATTERY: u8 = 0x55;
/// Report id do canal de firmware (usage page `0xFF22`), feature.
pub const REPORT_FIRMWARE: u8 = 0xAA;

/// Comando de consulta de bateria, dentro do report `0x55`.
const CMD_BATTERY: u8 = 0x65;

/// Tamanho de todo buffer trocado com o dispositivo: 1 byte de report id mais
/// os 63 de payload que o descritor declara. Comprimento e parte do contrato —
/// o kernel rejeita escrita de tamanho diferente.
pub const REPORT_LEN: usize = 64;

/// Espera obrigatoria entre o `SET_FEATURE` e o `GET_FEATURE` do `0xAA`.
/// Validado no hardware; ler antes disso devolve zeros.
pub const FEATURE_ROUNDTRIP_WAIT: Duration = Duration::from_millis(300);

/// Prazo para o dispositivo responder a consulta de bateria. E o default do
/// fabricante (`sendReportOnceSync`, `timeOut = 2e3`).
pub const BATTERY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);

/// De quem se quer a versao de firmware.
///
/// Sao dois, e so dois: o dongle 2.4GHz e o fone do outro lado dele.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareTarget {
    /// O receptor USB.
    Dongle,
    /// O fone.
    Headset,
}

impl FirmwareTarget {
    /// Byte que seleciona o alvo dentro do request.
    const fn selector(self) -> u8 {
        match self {
            Self::Dongle => 1,
            Self::Headset => 0,
        }
    }
}

/// Monta a consulta de bateria.
pub fn battery_request() -> [u8; REPORT_LEN] {
    let mut buf = [0u8; REPORT_LEN];
    buf[0] = REPORT_BATTERY;
    buf[1] = CMD_BATTERY;
    buf[2] = 0x01;
    buf
}

/// Monta a consulta de versao de firmware do dongle ou do fone.
pub fn firmware_request(target: FirmwareTarget) -> [u8; REPORT_LEN] {
    let mut buf = [0u8; REPORT_LEN];
    buf[0] = REPORT_FIRMWARE;
    buf[1] = 0x01;
    buf[2] = target.selector();
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_de_bateria_tem_64_bytes_e_o_prefixo_certo() {
        let req = battery_request();
        assert_eq!(req.len(), 64);
        assert_eq!(&req[..3], &[0x55, 0x65, 0x01]);
        assert!(req[3..].iter().all(|b| *b == 0));
    }

    #[test]
    fn request_de_firmware_distingue_dongle_de_fone() {
        let dongle = firmware_request(FirmwareTarget::Dongle);
        let headset = firmware_request(FirmwareTarget::Headset);
        assert_eq!(dongle.len(), 64);
        assert_eq!(headset.len(), 64);
        // Inverter esse byte faria o popover mostrar a versao do fone como se
        // fosse a do dongle: os dois valores sao plausiveis e ninguem pega.
        assert_eq!(&dongle[..3], &[0xAA, 0x01, 0x01]);
        assert_eq!(&headset[..3], &[0xAA, 0x01, 0x00]);
        assert!(dongle[3..].iter().all(|b| *b == 0));
        assert!(headset[3..].iter().all(|b| *b == 0));
    }

    #[test]
    fn constantes_de_tempo_sao_as_do_protocolo() {
        assert_eq!(
            FEATURE_ROUNDTRIP_WAIT,
            core::time::Duration::from_millis(300)
        );
        // 2 s e o default do fabricante (sendReportOnceSync: timeOut = 2e3),
        // nao os 3 s sem origem que o probe usava.
        assert_eq!(BATTERY_RESPONSE_TIMEOUT, core::time::Duration::from_secs(2));
    }
}
