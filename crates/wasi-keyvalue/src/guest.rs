//! # WASI Key-Value Guest

// Bindings for the `wasi:keyvalue` world.
// See (<https://github.com/WebAssembly/wasi-keyvalue/>)
wit_bindgen::generate!({
    world: "keyvalue",
    path: "wit",
    generate_all,
});

pub use self::wasi::keyvalue::*;
