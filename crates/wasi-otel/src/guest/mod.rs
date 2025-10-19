//! # WASI Bindings
//!
//! This module generates and exports WASI Guest bindings for local wit worlds.
//! The bindings are exported in as similar a manner to those in the Bytecode
//! Alliance's [wasi] crate.
//!
//! [wasi]: https://github.com/bytecodealliance/wasi

mod export;
mod init;

/// Bindings for the `wasi:blobstore` world.
mod generated {
    #![allow(clippy::future_not_send)]
    #![allow(clippy::collection_is_never_read)]

    wit_bindgen::generate!({
        world: "otel",
        path: "wit",
        generate_all,
        // pub_export_macro: true
    });
}

pub use wasi_otel_attr::instrument;

use self::init::Shutdown;

/// Initialize OpenTelemetry SDK and tracing subscriber.
pub fn init() -> Shutdown {
    match init::init() {
        Ok(shutdown) => shutdown,
        Err(e) => {
            ::tracing::error!("failed to initialize: {e}");
            Shutdown::default()
        }
    }
}
