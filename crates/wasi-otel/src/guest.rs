//! # WASI Bindings
//!
//! This module generates and exports WASI Guest bindings for local wit worlds.
//! The bindings are exported in as similar a manner to those in the Bytecode
//! Alliance's [wasi] crate.
//!
//! [wasi]: https://github.com/bytecodealliance/wasi

mod export;
mod init;

// Bindings for the `wasi:otel` world.
mod generated {
    #![allow(clippy::future_not_send)]
    #![allow(clippy::collection_is_never_read)]

    wit_bindgen::generate!({
        world: "otel",
        path: "wit",
        generate_all,
    });
}

/// Re-exported `instrument` macro for use in guest code.
pub use wasi_otel_attr::instrument;

use self::init::Shutdown;

/// Initialize OpenTelemetry SDK and tracing subscriber.
pub fn init() -> Shutdown {
    let shutdown = ::tracing::Span::current().is_none().then(init::init);

    match shutdown {
        Some(Ok(shutdown)) => shutdown,
        Some(Err(e)) => {
            ::tracing::error!("failed to initialize: {e}");
            Shutdown::default()
        }
        None => Shutdown::default(),
    }
    // match init::init() {
    //     Ok(shutdown) => shutdown,
    //     Err(e) => {
    //         ::tracing::error!("failed to initialize: {e}");
    //         Shutdown::default()
    //     }
    // }
}
