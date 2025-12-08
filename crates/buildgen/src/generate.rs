use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Type};

use crate::parse::BuildInput;

pub struct Generated {
    pub generate_main: bool,
    pub context_fields: Vec<TokenStream>,
    pub store_ctx_fields: Vec<TokenStream>,
    pub store_ctx_values: Vec<TokenStream>,
    pub host_trait_impls: Vec<Type>,
    pub server_trait_impls: Vec<TokenStream>,
    pub wasi_view_impls: Vec<TokenStream>,
}

impl TryFrom<BuildInput> for Generated {
    type Error = syn::Error;

    fn try_from(input: BuildInput) -> Result<Self, Self::Error> {
        // `Context` struct
        let mut context_fields = Vec::new();
        let mut seen_backends = HashSet::new();

        for backend in input.backends {
            // Deduplicate backends based on their string representation
            let backend_str = quote! {#backend}.to_string();
            if seen_backends.contains(&backend_str) {
                continue;
            }
            seen_backends.insert(backend_str);

            let field = field_name(&backend);
            context_fields.push(quote! {#field: #backend});
        }

        let mut store_ctx_fields = Vec::new();
        let mut store_ctx_values = Vec::new();
        let mut host_trait_impls = Vec::new();
        let mut server_trait_impls = Vec::new();
        let mut wasi_view_impls = Vec::new();

        for host in &input.hosts {
            let host_type = &host.type_;
            let host_name = quote! {#host_type}.to_string();
            let host_ident = syn::parse_str::<Ident>(&module_name(&host_name))?;
            let backend_type = &host.backend;
            let backend_ident = field_name(backend_type);

            host_trait_impls.push(host_type.clone());
            store_ctx_fields.push(quote! {#host_ident: #backend_type});
            store_ctx_values.push(quote! {#host_ident: self.#backend_ident.clone()});

            let module = &host_ident;

            // servers
            server_trait_impls.push(quote! {#host_type});

            // WasiViewXxx implementations
            let view = quote! {
                #module::wasi_view!(StoreCtx, #host_ident);
            };
            wasi_view_impls.push(view);
        }

        Ok(Self {
            generate_main: input.generate_main,
            context_fields,
            store_ctx_fields,
            store_ctx_values,
            host_trait_impls,
            server_trait_impls,
            wasi_view_impls,
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
    format_ident!("{name}")
}

fn module_name(s: &str) -> String {
    s.replace("Wasi", "wasi_").to_lowercase()
}
