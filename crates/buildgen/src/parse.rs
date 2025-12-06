use std::collections::HashSet;

use syn::parse::{Parse, ParseStream};
use syn::{Token, Type};

/// Configuration for the runtime macro.
///
/// Parses input in the form of 'host:backend' pairs. For example:
/// ```ignore
/// {
///     WasiHttp: WasiHttpCtxImpl,
///     WasiOtel: DefaultOtel,
///     ...
/// }
/// ```
pub struct BuildInput {
    pub hosts: Vec<Host>,
    pub backends: HashSet<Type>,
}

impl Parse for BuildInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::braced!(content in input);

        let mut backends: HashSet<Type> = HashSet::new();
        let mut hosts = Vec::<Host>::new();

        // parse 'host:backend' pairs
        while !content.is_empty() {
            let host_type = content.parse::<Type>()?;
            content.parse::<Token![:]>()?;
            let backend: Type = content.parse()?;

            let host_config = Host::new(host_type, backend.clone());
            hosts.push(host_config);
            backends.insert(backend);

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self { hosts, backends })
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
