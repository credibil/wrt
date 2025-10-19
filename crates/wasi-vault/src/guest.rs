//! # WASI Vault Guest

/// Bindings for the `wasi:vault` world.
/// See (<https://github.com/credibil/wasi-vault/>)
wit_bindgen::generate!({
    world: "vault",
    path: "wit",
    generate_all,
    pub_export_macro: true
});

pub use self::wasi::vault::*;
