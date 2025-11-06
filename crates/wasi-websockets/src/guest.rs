#![allow(clippy::future_not_send)]

//! # WASI WebSockets Guest

// Bindings for the `wasi:websockets` world.
// See (<https://github.com/credibil/wasi-websockets/>)
wit_bindgen::generate!({
    world: "websockets",
    path: "wit",
    generate_all,
    pub_export_macro: true
});

pub use self::wasi::websockets::*;
