//! # HTTP SDK
//!
//! Wasm component (guest) HTTP SDK.

mod cache;
mod error;
mod identity;
mod incoming;
mod outgoing;
mod uri;

pub use axum;

pub use self::error::*;
pub use self::incoming::*;
pub use self::outgoing::*;
pub use self::uri::*;

// Bindings for the `wasi:http` world.
wit_bindgen::generate!({
    world: "wasi:http/proxy",
    path: "wit",
    generate_all,
});

pub use self::wasi::http::*;
