//! Decodificacao da versao de firmware.

use crate::request::{MARK_FIRMWARE, REPORT_FIRMWARE};
use crate::{NoReading, reject};

/// Versao de firmware do dongle ou do fone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FirmwareVersion {
    /// Os quatro bytes crus, como vieram. Sao eles que se comparam quando se
    /// precisa de ordem.
    pub bytes: [u8; 4],
}

impl FirmwareVersion {
    /// Monta uma versao a partir dos quatro bytes crus. Existe pelo mesmo motivo
    /// que [`super::battery::BatteryReading::new`]: `#[non_exhaustive]` impede
    /// literal de struct fora deste crate.
    pub const fn new(bytes: [u8; 4]) -> Self {
        Self { bytes }
    }

    /// A forma textual que o fabricante usa: cada byte como um digito decimal.
    ///
    /// E **so exibicao**. Nao serve para comparar versoes: assim que um byte
    /// passa de 9 a concatenacao muda de comprimento e a ordem lexicografica
    /// deixa de significar qualquer coisa. Quem precisar comparar usa
    /// [`FirmwareVersion::bytes`].
    pub fn vendor_string(&self) -> String {
        self.bytes.iter().map(u8::to_string).collect()
    }
}

/// Le a resposta do feature report de firmware.
pub fn decode_firmware(buf: &[u8]) -> Result<FirmwareVersion, NoReading<'_>> {
    let ours = buf.first() == Some(&REPORT_FIRMWARE);
    // Os 6 primeiros bytes sao os que tem campo identificado neste canal.
    let reject = || reject(ours, buf, 6);

    if !ours || buf.get(1) != Some(&MARK_FIRMWARE) {
        return Err(reject());
    }

    // A versao ocupa os bytes 2..6.
    let Some(&bytes) = buf.get(2..).and_then(<[u8]>::first_chunk::<4>) else {
        return Err(reject());
    };

    Ok(FirmwareVersion { bytes })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Capturados do hardware em 11/09/2026.
    const DONGLE: &[u8] = &[0xAA, 0x01, 0x00, 0x00, 0x01, 0x02, 0xFF, 0x25];
    const HEADSET: &[u8] = &[0xAA, 0x01, 0x00, 0x00, 0x03, 0x06, 0xFF, 0x25];

    #[test]
    fn decodifica_os_pacotes_reais_do_hardware() {
        let d = decode_firmware(DONGLE).expect("pacote real do dongle");
        assert_eq!(d.bytes, [0x00, 0x00, 0x01, 0x02]);
        assert_eq!(d.vendor_string(), "0012");

        let h = decode_firmware(HEADSET).expect("pacote real do fone");
        assert_eq!(h.bytes, [0x00, 0x00, 0x03, 0x06]);
        assert_eq!(h.vendor_string(), "0036");
    }

    #[test]
    fn prefixo_errado_e_rejeitado() {
        // Sem o report id no byte 0 — a convencao do WebHID, que ja causou um
        // erro de indice durante a investigacao.
        assert!(decode_firmware(&[0x01, 0x00, 0x00, 0x01, 0x02]).is_err());
        // Report id certo, segundo byte errado.
        assert!(decode_firmware(&[0xAA, 0x02, 0x00, 0x00, 0x01, 0x02]).is_err());
        // Canal de bateria nao e canal de firmware.
        assert!(decode_firmware(&[0x55, 0x65, 0x46, 0x02]).is_err());
    }

    #[test]
    fn buffers_curtos_nao_entram_em_panico() {
        // O campo de versao vai ate o indice 5: qualquer coisa menor estoura
        // numa indexacao direta.
        for n in 0..6usize {
            assert!(decode_firmware(&DONGLE[..n]).is_err(), "n = {n}");
        }
        assert!(decode_firmware(&DONGLE[..6]).is_ok());
    }

    #[test]
    fn byte_acima_de_nove_mostra_por_que_a_forma_textual_nao_ordena() {
        let v = decode_firmware(&[0xAA, 0x01, 0x00, 0x00, 0x00, 0xFF]).expect("prefixo valido");
        assert_eq!(v.bytes, [0x00, 0x00, 0x00, 0xFF]);
        // A concatenacao do fabricante vira "000255": mais longa que "0012" e
        // portanto sem ordem util como string. Por isso comparacao usa bytes.
        assert_eq!(v.vendor_string(), "000255");
    }
}
