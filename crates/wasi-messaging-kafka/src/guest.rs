#![allow(clippy::future_not_send)]
#![allow(clippy::collection_is_never_read)]

//! # WASI Messaging Guest

// Bindings for the `wasi:messaging` world.
// See (<https://github.com/WebAssembly/wasi-messaging/>)
wit_bindgen::generate!({
    world: "messaging",
    path: "../wasi-messaging/wit",
    additional_derives: [Clone],
    generate_all,
    pub_export_macro: true
});

pub use self::exports::wasi::messaging::*;
pub use self::wasi::messaging::*;
