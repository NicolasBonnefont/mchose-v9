//! Applet COSMIC com a bateria do MCHOSE V9 PRO.

#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

pub mod app;
pub mod state;
