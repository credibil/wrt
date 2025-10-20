//! # WASI OpenTelemetry
//!
//! Bindings for the OpenTelemetry specification (wasi:otel) for guest and host
//! components.

#[cfg(target_arch = "wasm32")]
mod guest;
#[cfg(target_arch = "wasm32")]
pub use guest::*;

#[cfg(unix)]
mod host;
#[cfg(unix)]
pub use host::*;
