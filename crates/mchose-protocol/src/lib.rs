//! Protocolo HID do headset MCHOSE V9 PRO (USB `291d:385d`).
//!
//! Converte bytes em tipos e tipos em bytes. Nao abre arquivo, nao faz syscall
//! e nao sabe que `/dev/hidraw` existe: quem fala com o dispositivo e o crate
//! `mchose-device`.
//!
//! O protocolo foi extraido do driver oficial e validado contra o hardware.
//! Ver `docs/specs/1-mchose-protocol/spec.md`.

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

pub mod device;
