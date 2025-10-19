//! # WASI Bindings
//!
//! This module generates and exports WASI Guest bindings for local wit worlds.
//! The bindings are exported in as similar a manner to those in the Bytecode
//! Alliance's [wasi] crate.
//!
//! [wasi]: https://github.com/bytecodealliance/wasi

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
