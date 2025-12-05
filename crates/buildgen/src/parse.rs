use std::collections::HashSet;

use quote::quote;
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
    pub is_server: bool,
}

impl Host {
    fn new(type_: Type, backend: Type) -> Self {
        let name = quote! {#type_}.to_string();
        let short_name = name.strip_prefix("Wasi").unwrap_or(&name).to_lowercase();
        let is_server = matches!(short_name.as_str(), "http" | "messaging" | "websockets");

        Self {
            type_,
            backend,
            is_server,
        }
    }
}

// fn is_server<S: kernel::State, T: kernel::Server<S>>() {
//     true
// }
