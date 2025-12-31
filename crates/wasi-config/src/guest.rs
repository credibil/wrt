//! # WASI Config WIT implementation

#![allow(clippy::same_length_and_capacity)]

// Bindings for the `wasi:config` world.
// See (<https://github.com/WebAssembly/wasi-config/>)
wit_bindgen::generate!({
    world: "config",
    path: "wit",
    generate_all,
});

pub use self::wasi::config::*;
