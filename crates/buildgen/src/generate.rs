use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Type};

use crate::parse::BuildInput;

pub struct Generated {
    context: Context,
    store_ctx: StoreCtx,
    servers: Vec<TokenStream>,
    wasi_views: Vec<TokenStream>,
}

impl From<BuildInput> for Generated {
    fn from(input: BuildInput) -> Self {
        let mut servers = Vec::new();
        let mut wasi_views = Vec::new();

        for host in &input.hosts {
            let type_ = &host.type_;
            let host_name = quote! {#type_}.to_string();
            let host_ident = format_ident!("{}", &snake_case(&host_name));

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

        Self {
            context: Context::from(&input.backends),
            store_ctx: StoreCtx::from(&input),
            servers,
            wasi_views,
        }
    }
}

pub struct Context {
    pub fields: Vec<TokenStream>,
}

impl From<&HashSet<Type>> for Context {
    fn from(backends: &HashSet<Type>) -> Self {
        let mut fields = Vec::new();
        for backend in backends {
            let field = field_name(backend);
            fields.push(quote! {#field: #backend});
        }
        Self { fields }
    }
}

pub struct StoreCtx {
    fields: Vec<TokenStream>,
    instance: Vec<TokenStream>,
}

impl From<&BuildInput> for StoreCtx {
    fn from(input: &BuildInput) -> Self {
        let mut fields = Vec::new();
        let mut instance = Vec::new();

        for host in &input.hosts {
            let type_ = &host.type_;
            let host_name = quote! {#type_}.to_string();
            let host_ident = format_ident!("{}", &snake_case(&host_name));

            let backend_type = &host.backend;
            fields.push(quote! {pub #host_ident: #backend_type});

            let backend_ident = field_name(&host.backend);
            instance.push(quote! {#host_ident: self.#backend_ident.clone()});
        }
        Self { fields, instance }
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
