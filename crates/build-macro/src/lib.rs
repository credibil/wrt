//! # Build Macros
mod buildgen;

use syn::{Error, parse_macro_input};

#[proc_macro]
pub fn buildgen(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    buildgen::expand(&parse_macro_input!(input as buildgen::Config))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
