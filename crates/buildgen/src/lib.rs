mod expand;
mod parse;
// mod generate;

use proc_macro::TokenStream;
use syn::parse_macro_input;

use self::expand::{PreExpand, expand};
use self::parse::BuildInput;

/// Generates the runtime infrastructure based on the configuration.
///
/// # Example
///
/// ```ignore
/// buildgen::runtime!({
///     wasi_http: WasiHttp,
///     wasi_otel: DefaultOtel,
///     wasi_blobstore: MongoDb,
/// });
/// ```
#[proc_macro]
pub fn runtime(input: TokenStream) -> TokenStream {
    let config = parse_macro_input!(input as BuildInput);
    let pre_expand = match PreExpand::try_from(config) {
        Ok(pre_expand) => pre_expand,
        Err(e) => return e.into_compile_error().into(),
    };
    expand(pre_expand).into()
}
