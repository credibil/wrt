//! # WASI Vault Guest

#![allow(clippy::future_not_send)]

// Bindings for the `wasi:vault` world.
// See (<https://github.com/credibil/wasi-vault/>)
wit_bindgen::generate!({
    world: "identity",
    path: "wit",
    generate_all,
});

pub use self::wasi::identity::*;
