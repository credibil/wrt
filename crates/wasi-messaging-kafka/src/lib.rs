//! # WASI Messaging with Kafka
//!
//! TODO: This implementation is intended to live only until a more successful
//! templated solution for `wasi:messaging` is developed that allows for the
//! complex lifetime management required by Kafka producers and consumers. A
//! solution is required that allows for the use of, say, NATS and Kafka
//! interchangeably by simply swapping out the resources.
//!
//! This module implements a runtime service for `wasi:messaging`
//! (<https://github.com/WebAssembly/wasi-messaging>).

#[cfg(target_arch = "wasm32")]
mod guest;
#[cfg(target_arch = "wasm32")]
pub use guest::*;

#[cfg(not(target_arch = "wasm32"))]
mod host;
#[cfg(not(target_arch = "wasm32"))]
pub use host::*;
