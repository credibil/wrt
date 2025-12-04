use std::collections::HashSet;

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Token, Type};

/// Configuration for the runtime macro.
///
/// Parses input in the form of 'host:backend' pairs. For example:
/// ```ignore
/// {
///     WasiHttp: DefaultHttp,
///     WasiOtel: DefaultOtel,
///     ...
/// }
/// ```
pub struct BuildInput {
    pub hosts: Vec<HostConfig>,
    pub backends: HashSet<Type>,
}

impl Parse for BuildInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::braced!(content in input);

        let mut backends: HashSet<Type> = HashSet::new();
        let mut hosts = Vec::<HostConfig>::new();

        // parse 'host:backend' pairs
        while !content.is_empty() {
            let host_type = content.parse::<Type>()?;
            content.parse::<Token![:]>()?;
            let backend: Type = content.parse()?;

            let host_config = HostConfig::new(host_type, backend.clone());
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
pub struct HostConfig {
    pub host_type: Type,
    pub backend: Type,
    pub is_server: bool,
}

impl HostConfig {
    fn new(host_type: Type, backend: Type) -> Self {
        let name = quote! {#host_type}.to_string();
        let short_name = name.strip_prefix("Wasi").unwrap_or(&name).to_lowercase();
        let is_server = matches!(short_name.as_str(), "http" | "messaging" | "websockets");

        Self {
            host_type,
            backend,
            is_server,
        }
    }
}
