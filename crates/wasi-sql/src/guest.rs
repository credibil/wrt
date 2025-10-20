//! # WASI SQL Guest

// Bindings for the `wasi:sql` world.
// See (<https://github.com/credibil/wasi-sql/>)
wit_bindgen::generate!({
    world: "sql",
    path: "wit",
    generate_all,
    pub_export_macro: true
});

pub use self::wasi::sql::*;
