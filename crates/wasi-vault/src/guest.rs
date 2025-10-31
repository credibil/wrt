//! # WASI Vault Guest

// Bindings for the `wasi:vault` world.
// See (<https://github.com/credibil/wasi-vault/>)
wit_bindgen::generate!({
    world: "vault",
    path: "wit",
    generate_all,
});

pub use self::wasi::vault::*;
