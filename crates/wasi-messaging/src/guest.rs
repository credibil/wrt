//! # WASI Messaging Guest

/// Bindings for the `wasi:messaging` world.
/// See (<https://github.com/WebAssembly/wasi-messaging/>)
wit_bindgen::generate!({
    world: "messaging",
    path: "wit",
    additional_derives: [Clone],
    generate_all,
    pub_export_macro: true
});

pub use self::exports::wasi::messaging::*;
pub use self::wasi::messaging::*;
