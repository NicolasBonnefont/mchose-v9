//! EQ de 10 bandas para o MCHOSE V9 PRO.
//!
//! O EQ **nao e comando do dispositivo**: no Windows ele e um APO, DSP rodando
//! no host. No Linux o equivalente e a `filter-chain` do PipeWire, e e isso que
//! este crate opera.

// `forbid`, e nao `deny`: aqui nao ha ioctl, e a invariante do spec proibe FFI.
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

pub mod config;
pub mod install;
pub mod pipewire;
