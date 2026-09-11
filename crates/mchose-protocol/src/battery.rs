//! Decodificacao da resposta de bateria.
//!
//! O mesmo decodificador serve a resposta solicitada e ao push espontaneo: o
//! fone manda esse report sozinho quando o estado muda, entao o consumidor
//! normal e passivo, sem polling.

use crate::request::{CMD_BATTERY, REPORT_BATTERY};
use crate::{NoReading, reject};

/// Estado de carga informado pelo fone.
///
/// `#[non_exhaustive]` porque o protocolo continua sendo mapeado: um estado
/// novo nao pode quebrar quem consome este crate de surpresa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChargeState {
    /// Em uso, fora do carregador.
    Discharging,
    /// Carregando. Raro de observar: o fone se desconecta do dongle 2.4GHz
    /// quando e plugado para carregar, entao na pratica ele some em vez de
    /// reportar este estado.
    Charging,
    /// Bateria cheia.
    Full,
    /// Dormindo.
    Asleep,
    /// Byte que o firmware mandou e que ainda nao sabemos ler. Preservado de
    /// proposito: e assim que o protocolo continua sendo mapeado.
    Unknown(u8),
}

impl ChargeState {
    const fn from_byte(byte: u8) -> Self {
        match byte {
            2 => Self::Discharging,
            3 => Self::Charging,
            4 => Self::Full,
            26 => Self::Asleep,
            outro => Self::Unknown(outro),
        }
    }
}

/// Uma leitura valida de bateria.
///
/// `#[non_exhaustive]` impede literal de struct fora deste crate, entao quem
/// precisa construir uma — um teste do applet, por exemplo — usa
/// [`BatteryReading::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BatteryReading {
    /// Carga em porcentagem, sempre dentro de `0..=100`.
    pub percent: u8,
    /// Estado de carga.
    pub state: ChargeState,
}

impl BatteryReading {
    /// Monta uma leitura. Existe para consumidores poderem construir casos de
    /// teste; o percentual nao e validado aqui, quem valida e o decodificador.
    pub const fn new(percent: u8, state: ChargeState) -> Self {
        Self { percent, state }
    }
}

/// Le a resposta de bateria.
pub fn decode_battery(buf: &[u8]) -> Result<BatteryReading, NoReading<'_>> {
    // Do nosso canal ou nao? E o que decide se a rejeicao merece registro.
    // Buffer vazio cai aqui como "nao e nosso", que e o desejado.
    let ours = buf.first() == Some(&REPORT_BATTERY);
    // Os 4 primeiros bytes sao os que tem campo identificado neste canal.
    let reject = || reject(ours, buf, 4);

    if !ours || buf.get(1) != Some(&CMD_BATTERY) {
        return Err(reject());
    }

    let (Some(&percent), Some(&state)) = (buf.get(2), buf.get(3)) else {
        return Err(reject());
    };

    // Numero errado na tela e pior que ausencia de numero.
    if percent > 100 {
        return Err(reject());
    }

    Ok(BatteryReading {
        percent,
        state: ChargeState::from_byte(state),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Capturado do hardware em 11/09/2026.
    const REAL: &[u8] = &[0x55, 0x65, 0x46, 0x02, 0, 0, 0, 0];

    #[test]
    fn decodifica_o_pacote_real_do_hardware() {
        let r = decode_battery(REAL).expect("pacote real deve decodificar");
        assert_eq!(r.percent, 70);
        assert_eq!(r.state, ChargeState::Discharging);
    }

    #[test]
    fn mapeia_os_estados_conhecidos() {
        for (byte, esperado) in [
            (2u8, ChargeState::Discharging),
            (3, ChargeState::Charging),
            (4, ChargeState::Full),
            (26, ChargeState::Asleep),
        ] {
            let buf = [0x55, 0x65, 50, byte];
            assert_eq!(decode_battery(&buf).expect("valido").state, esperado);
        }
    }

    #[test]
    fn estado_desconhecido_preserva_o_percentual_e_expoe_o_byte() {
        let buf = [0x55, 0x65, 42, 99];
        let r = decode_battery(&buf).expect("percentual valido, estado estranho");
        assert_eq!(r.percent, 42);
        assert_eq!(r.state, ChargeState::Unknown(99));
    }

    #[test]
    fn percentual_fora_de_faixa_invalida_e_devolve_os_bytes() {
        let buf = [0x55, 0x65, 200, 0x02];
        let e = decode_battery(&buf).expect_err("200% nao e leitura");
        assert_eq!(e.rejected, Some(&buf[..]));
    }

    #[test]
    fn trafego_alheio_e_descartado_sem_virar_registro() {
        // Tecla de midia (Consumer, report 0x06) chega no mesmo /dev/hidraw a
        // cada toque de volume. Nao e anomalia.
        let e = decode_battery(&[0x06, 0x01, 0x00]).expect_err("nao e nosso");
        assert_eq!(e.rejected, None);
        // Telefonia.
        assert_eq!(
            decode_battery(&[0x07, 0x01])
                .expect_err("nao e nosso")
                .rejected,
            None
        );
    }

    #[test]
    fn buffer_sem_report_id_e_rejeitado_e_nao_vira_2_porcento() {
        let e = decode_battery(&[0x65, 0x46, 0x02, 0x00]).expect_err("sem report id");
        assert_eq!(e.rejected, None);
    }

    #[test]
    fn buffers_curtos_nao_entram_em_panico() {
        for n in 0..4usize {
            let buf = &REAL[..n];
            let e = decode_battery(buf).expect_err("curto demais");
            // Truncado mas do nosso canal merece registro; vazio nao identifica.
            let esperado = if n == 0 { None } else { Some(buf) };
            assert_eq!(e.rejected, esperado);
        }
    }

    #[test]
    fn buffer_maior_que_o_esperado_decodifica_ignorando_o_excedente() {
        let mut buf = [0u8; 128];
        buf[0] = 0x55;
        buf[1] = 0x65;
        buf[2] = 70;
        buf[3] = 0x02;
        let r = decode_battery(&buf).expect("prefixo valido");
        assert_eq!(r.percent, 70);
    }
}
