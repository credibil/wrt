//! # WASI Blobstore Service
//!
//! This module implements a runtime service for `wasi:blobstore`
//! (<https://github.com/WebAssembly/wasi-blobstore>).

#[cfg(target_arch = "wasm32")]
mod guest;
#[cfg(target_arch = "wasm32")]
pub use guest::*;

#[cfg(unix)]
mod host;
#[cfg(unix)]
pub use host::*;
