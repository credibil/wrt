#![allow(clippy::future_not_send)]
#![allow(clippy::collection_is_never_read)]

//! # WASI Messaging Guest

// Bindings for the `wasi:messaging` world.
// See (<https://github.com/WebAssembly/wasi-messaging/>)
wit_bindgen::generate!({
    world: "messaging",
    path: "wit",
    additional_derives: [Clone],
    generate_all,
    pub_export_macro: true,
    // async: [
    //     "wasi:messaging/producer@0.2.0-draft#send",
    //     "wasi:messaging/request-reply@0.2.0-draft#request",
    //     "wasi:messaging/incoming-handler@0.2.0-draft#handle",
    // ],
});

pub use self::exports::wasi::messaging::*;
pub use self::wasi::messaging::*;
