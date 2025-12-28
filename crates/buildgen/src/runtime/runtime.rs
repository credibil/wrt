pub mod expand;
pub mod generate;

use syn::parse::{Parse, ParseStream};
use syn::{Token, Type};

mod kw {
    syn::custom_keyword!(main);
}

/// Configuration for the runtime macro.
///
/// Parses input in the form of 'host:backend' pairs. For example:
/// ```ignore
/// {
///     WasiHttp: HttpDefault,
///     WasiOtel: DefaultOtel,
///     ...
/// }
/// ```
pub struct BuildInput {
    pub gen_main: bool,
    pub hosts: Vec<Host>,
    pub backends: Vec<Type>,
}

impl Parse for BuildInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let gen_main = if input.peek(kw::main) {
            input.parse::<kw::main>()?;
            input.parse::<Token![,]>()?;
            true
        } else {
            false
        };

        let content;
        syn::braced!(content in input);

        let mut backends = Vec::new();
        let mut hosts = Vec::<Host>::new();

        // parse 'host:backend' pairs
        while !content.is_empty() {
            let host_type = content.parse::<Type>()?;
            content.parse::<Token![:]>()?;
            let backend: Type = content.parse()?;

            let host_config = Host::new(host_type, backend.clone());
            hosts.push(host_config);
            backends.push(backend);

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            gen_main,
            hosts,
            backends,
        })
    }
}

/// Information about a WASI host and its configuration.
pub struct Host {
    pub type_: Type,
    pub backend: Type,
}

impl Host {
    const fn new(type_: Type, backend: Type) -> Self {
        Self { type_, backend }
    }
}
