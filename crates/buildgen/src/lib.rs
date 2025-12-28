mod guest;
mod runtime;

use proc_macro::TokenStream;
use syn::parse_macro_input;

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
    let parsed = parse_macro_input!(input as runtime::RuntimeInput);
    let generated = match crate::runtime::generate::Generated::try_from(parsed) {
        Ok(generated) => generated,
        Err(e) => return e.into_compile_error().into(),
    };
    crate::runtime::expand::expand(generated).into()
}

/// Generates the guest infrastructure based on the configuration.
///
/// # Example
///
/// ```ignore
/// buildgen::guest!({
///     http: [
///         "/jobs/detector": {
///             method: get,
///             request: String,
///             handler: detection_request,
///             response: DetectionResponse,
///         }
///     ],
///     messaging: [
///         "realtime-r9k.v1": {
///             message: R9kMessage,
///         }
///     ]
/// });
/// ```
#[proc_macro]
pub fn guest(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as guest::GuestInput);
    let generated = match crate::guest::generate::Generated::try_from(parsed) {
        Ok(generated) => generated,
        Err(e) => return e.into_compile_error().into(),
    };
    crate::guest::expand::expand(generated).into()
}
