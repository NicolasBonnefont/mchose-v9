//! Protocolo HID do headset MCHOSE V9 PRO (USB `291d:385d`).
//!
//! Converte bytes em tipos e tipos em bytes. Nao abre arquivo, nao faz syscall
//! e nao sabe que `/dev/hidraw` existe: quem fala com o dispositivo e o crate
//! `mchose-device`.
//!
//! O protocolo foi extraido do driver oficial e validado contra o hardware.
//! Ver `README.md` e `crates/mchose-protocol/AGENTS.md`.

#![forbid(unsafe_code)]
// Os lints abaixo sao o que sustenta a invariante "nenhuma entrada de qualquer
// tamanho causa panico": todo byte que vem do dispositivo e acessado com
// `get`, nunca com indexacao direta. Ficam desligados em `cfg(test)` porque
// `assert_eq!` expande para `panic!`.
#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

/// Ausencia de leitura.
///
/// Prefixo que nao casa, pacote curto demais e conteudo fora de faixa sao a
/// mesma coisa para quem consome: nao ha numero para mostrar. Por isso um tipo
/// so, e nao uma taxonomia que o chamador teria de destrinchar para sempre
/// tomar a mesma decisao.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct NoReading<'a> {
    /// Bytes do pacote rejeitado, quando ele era de um canal nosso e portanto
    /// merece registro.
    ///
    /// `None` quando o pacote nem era nosso: o mesmo `/dev/hidraw` carrega
    /// teclas de midia e telefonia, que chegam a cada toque de volume. Tratar
    /// isso como anomalia encheria o log em uso normal e afogaria o byte que
    /// importa.
    pub rejected: Option<&'a [u8]>,
}

/// Monta a rejeicao, unica para os dois decodificadores.
///
/// `known` e quantos bytes daquele canal tem campo identificado. O resto do
/// buffer de 64 B nunca foi mapeado, e quem registra em log costuma colar a
/// linha num issue — de um repositorio publico. Registrar byte de significado
/// desconhecido e o que a invariante "fixture e byte de protocolo, nao captura
/// de sessao" existe para impedir.
pub(crate) fn reject(ours: bool, buf: &[u8], known: usize) -> NoReading<'_> {
    NoReading {
        rejected: ours.then(|| buf.get(..known).unwrap_or(buf)),
    }
}

pub mod battery;
pub mod device;
pub mod firmware;
pub mod request;
