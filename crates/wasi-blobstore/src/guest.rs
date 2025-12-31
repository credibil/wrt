//! # WASI Blobstore WIT implementation

#![allow(clippy::same_length_and_capacity)]

// Bindings for the `wasi:blobstore` world.
// See (<https://github.com/WebAssembly/wasi-blobstore/>)
wit_bindgen::generate!({
    world: "blobstore",
    path: "wit",
    generate_all,
});

pub use self::wasi::blobstore::*;
