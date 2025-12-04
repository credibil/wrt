use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Type};

use crate::parse::BuildInput;

pub struct Generated {
    pub context: Vec<TokenStream>,
    pub host_types: Vec<Type>,
    pub host_idents: Vec<Ident>,
    pub backend_types: Vec<Type>,
    pub backend_idents: Vec<Ident>,
    pub servers: Vec<TokenStream>,
    pub wasi_views: Vec<TokenStream>,
}

impl TryFrom<BuildInput> for Generated {
    type Error = syn::Error;

    fn try_from(input: BuildInput) -> Result<Self, Self::Error> {
        // `Context` struct
        let mut context = Vec::new();
        for backend in input.backends {
            let field = field_name(&backend);
            context.push(quote! {#field: #backend});
        }

        let mut host_types = Vec::new();
        let mut host_idents = Vec::new();
        let mut backend_types = Vec::new();
        let mut backend_idents = Vec::new();
        let mut servers = Vec::new();
        let mut wasi_views = Vec::new();

        for host in &input.hosts {
            let type_ = &host.type_;
            let host_name = quote! {#type_}.to_string();
            let host_ident = syn::parse_str::<Ident>(&snake_case(&host_name))?;

            host_types.push(type_.clone());
            host_idents.push(host_ident.clone());
            backend_types.push(host.backend.clone());
            backend_idents.push(field_name(&host.backend));

            let module = &host_ident;

            // servers
            if host.is_server {
                let start = quote! {Box::pin(#module::#type_.run(self))};
                servers.push(start);
            }

            // WasiViewXxx implementations
            let short_name = host_name.strip_prefix("Wasi").unwrap_or(&host_name).to_lowercase();
            let view_trait = format_ident!("{host_name}View");
            let view_method = format_ident!("{short_name}");
            let ctx_view = format_ident!("{host_name}CtxView");

            let view = quote! {
                impl #module::#view_trait for StoreCtx {
                    fn #view_method(&mut self) -> #module::#ctx_view<'_> {
                        #module::#ctx_view {
                            ctx: &mut self.#host_ident,
                            table: &mut self.table,
                        }
                    }
                }
            };
            wasi_views.push(view);
        }

        Ok(Self {
            context,
            host_types,
            host_idents,
            backend_types,
            backend_idents,
            servers,
            wasi_views,
        })
    }
}

/// Generates a field name for a backend type.
fn field_name(field_type: &Type) -> Ident {
    let name = if let Type::Path(type_path) = field_type {
        type_path.path.segments.last().map_or_else(
            || "backend".to_string(),
            |segment| segment.ident.to_string().to_lowercase(),
        )
    } else {
        "backend".to_string()
    };

    syn::parse_str(&name).unwrap()
}

fn snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}
