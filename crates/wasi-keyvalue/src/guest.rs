//! # WASI Key-Value Guest

// Bindings for the `wasi:keyvalue` world.
// See (<https://github.com/WebAssembly/wasi-keyvalue/>)
wit_bindgen::generate!({
    world: "keyvalue",
    path: "wit",
    generate_all,
});

mod cache;

pub use self::cache::*;
pub use self::wasi::keyvalue::*;
