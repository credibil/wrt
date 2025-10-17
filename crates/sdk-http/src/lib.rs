//! # HTTP SDK
//!
//! Wasm component (guest) HTTP SDK.

// Bindings for the `wasi:keyvalue` world.
/// See (<https://github.com/WebAssembly/wasi-keyvalue/>)
pub mod generated {
    wit_bindgen::generate!({
        world: "keyvalue",
        path: "wit",
        generate_all,
        pub_export_macro: true
    });
}

mod cache;
mod client;
mod error;
mod router;
mod uri;

pub use axum;

pub use self::client::*;
pub use self::error::*;
pub use self::router::serve;
pub use self::uri::*;
