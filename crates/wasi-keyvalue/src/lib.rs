//! # WASI Key-Value
//!
//! This module implements a runtime service for `wasi:keyvalue`
//! (<https://github.com/WebAssembly/wasi-keyvalue>).

#[cfg(target_arch = "wasm32")]
mod guest;
#[cfg(target_arch = "wasm32")]
pub use guest::*;

#[cfg(not(target_arch = "wasm32"))]
mod host;
#[cfg(not(target_arch = "wasm32"))]
pub use host::*;
