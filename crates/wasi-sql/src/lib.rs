//! # WASI Vault Service
//!
//! This module implements a runtime service for `wasi:vault`
//! (<https://github.com/credibil/wasi-vault>).

#[cfg(target_arch = "wasm32")]
mod guest;
#[cfg(target_arch = "wasm32")]
pub use guest::*;

#[cfg(unix)]
mod host;
#[cfg(unix)]
pub use host::*;
