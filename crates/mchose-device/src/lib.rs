//! Acesso ao headset MCHOSE V9 PRO pelo `/dev/hidraw`, com hotplug.
//!
//! Descobre o dispositivo, mantem um fluxo de eventos vivo enquanto o dongle
//! existir, e sobrevive a ele sumir e voltar. Nao conhece `iced` nem
//! `libcosmic`: quem embrulha o `Stream` numa `Subscription` e o applet.
//!
//! Ver `docs/specs/2-mchose-device/spec.md`.

// `deny`, nao `forbid`: os dois ioctls de feature do hidraw nao tem involucro
// seguro na std, entao o modulo de transporte real precisa de `unsafe`. `deny`
// permite abrir a excecao num lugar so e deixa o resto do crate fechado;
// `forbid` nao poderia ser aberto nem com justificativa.
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
