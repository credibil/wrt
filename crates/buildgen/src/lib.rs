mod expand;
mod generate;
mod parse;

use proc_macro::TokenStream;
use syn::parse_macro_input;

use self::expand::expand;
use self::generate::Generated;
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
    let parsed = parse_macro_input!(input as BuildInput);
    let generated = match Generated::try_from(parsed) {
        Ok(generated) => generated,
        Err(e) => return e.into_compile_error().into(),
    };
    expand(generated).into()
}
