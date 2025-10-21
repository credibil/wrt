//! # WASI Blobstore Service
//!
//! This module implements a runtime service for `wasi:blobstore`
//! (<https://github.com/WebAssembly/wasi-blobstore>).

#[cfg(target_arch = "wasm32")]
mod guest;
#[cfg(target_arch = "wasm32")]
pub use guest::*;

#[cfg(not(target_arch = "wasm32"))]
mod host;
#[cfg(not(target_arch = "wasm32"))]
pub use host::*;
