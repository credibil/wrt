//! # WASI Bindings
//!
//! This module generates and exports WASI Guest bindings for local wit worlds.
//! The bindings are exported in as similar a manner to those in the Bytecode
//! Alliance's [wasi] crate.
//!
//! [wasi]: https://github.com/bytecodealliance/wasi

mod convert;
mod init;
#[cfg(feature = "metrics")]
mod metrics;
#[cfg(feature = "tracing")]
mod tracing;

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

use self::init::ExitGuard;

/// Initialize OpenTelemetry SDK and tracing subscriber.
pub fn init() -> Option<ExitGuard> {
    if init::INIT.read().is_ok_and(|x| *x) {
        return None;
    }

    match init::init() {
        Ok(guard) => Some(guard),
        Err(e) => {
            ::tracing::error!("failed to initialize: {e}");
            None
        }
    }
}
