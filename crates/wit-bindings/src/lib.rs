//! # WASI Bindings
//!
//! This module generates and exports WASI Guest bindings for local wit worlds.
//! The bindings are exported in as similar a manner to those in the Bytecode
//! Alliance's [wasi] crate.
//!
//! [wasi]: https://github.com/bytecodealliance/wasi

/// Bindings for the `wasi:blobstore` world.
/// See (<https://github.com/WebAssembly/wasi-blobstore/>)
pub mod blobstore {
    pub use self::wasi::blobstore::*;

    wit_bindgen::generate!({
        world: "blobstore",
        path: "wit",
        generate_all,
        pub_export_macro: true
    });
}

/// Bindings for the `wasi:sql` world.
pub mod sql {
    pub use self::wasi::sql::*;

    wit_bindgen::generate!({
        world: "sql",
        path: "wit",
        generate_all,
        pub_export_macro: true
    });
}

/// Bindings for the `wasi:vault` world.
pub mod vault {
    pub use self::wasi::vault::*;

    wit_bindgen::generate!({
        world: "vault",
        path: "wit",
        generate_all,
        pub_export_macro: true
    });
}
