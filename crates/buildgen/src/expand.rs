use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

use crate::parse::BuildInput;

pub struct PreExpand {
    context_fields: Vec<TokenStream>,
    context_inits: Vec<TokenStream>,
    store_ctx_struct: Vec<TokenStream>,
    store_ctx_data: Vec<TokenStream>,
    link_interfaces: Vec<TokenStream>,
    server_starts: Vec<TokenStream>,
    view_impls: Vec<TokenStream>,
}

impl TryFrom<BuildInput> for PreExpand {
    type Error = syn::Error;

    fn try_from(input: BuildInput) -> Result<Self, Self::Error> {
        // `RuntimeContext` struct fields & `new()` method initialization
        let mut context_fields = Vec::new();
        let mut context_inits = Vec::new();

        for backend in input.backends {
            let field_ident = field_name(&backend);
            let field = quote! {
                #field_ident: #backend
            };
            let init = quote! {
                #field_ident: #backend::connect().await?
            };

            context_fields.push(field);
            context_inits.push(init);
        }

        // `StoreCtx` fields
        let mut store_ctx_struct = Vec::new();
        let mut store_ctx_data = Vec::new();
        let mut link_interfaces = Vec::new();
        let mut server_starts = Vec::new();

        for host in &input.hosts {
            let host_type = &host.host_type;
            let name = quote! {#host_type}.to_string();
            let field_ident = syn::parse_str::<Ident>(&snake_case(&name))?;

            // `StoreCtx` struct fields
            let backend_ty = &host.backend;
            let field = quote! {
                pub #field_ident: #backend_ty
            };
            store_ctx_struct.push(field);

            // `StoreCtx` data fields
            let backend_ident = field_name(backend_ty);
            let init = quote! {
                #field_ident: self.#backend_ident.clone()
            };
            store_ctx_data.push(init);

            // link interfaces
            let host_type = &host.host_type;
            link_interfaces.push(quote! {
                compiled.link(#host_type)?;
            });

            // servers
            if host.is_server {
                let module_name = snake_case(&name);
                let module = syn::parse_str::<Ident>(&module_name)?;
                let start = quote! {
                    Box::pin(#module::#host_type.run(self))
                };
                server_starts.push(start);
            }
        }

        // WasiViewXxx implementations
        let view_impls: Vec<_> = input
            .hosts
            .iter()
            .map(|host| {
                let host_type = &host.host_type;
                let name = quote! {#host_type}.to_string();
                let field_ident = syn::parse_str::<Ident>(&snake_case(&name)).unwrap();

                let short_name = name.strip_prefix("Wasi").unwrap_or(&name).to_lowercase();

                let view_trait = syn::parse_str::<Ident>(&format!("{name}View")).unwrap();
                let view_method = syn::parse_str::<Ident>(&short_name).unwrap();
                let ctx_view = syn::parse_str::<Ident>(&format!("{name}CtxView")).unwrap();

                let module_name = snake_case(&name);
                let module = syn::parse_str::<Ident>(&module_name).unwrap();

                quote! {
                    impl #module::#view_trait for StoreCtx {
                        fn #view_method(&mut self) -> #module::#ctx_view<'_> {
                            #module::#ctx_view {
                                ctx: &mut self.#field_ident,
                                table: &mut self.table,
                            }
                        }
                    }
                }
            })
            .collect();

        Ok(Self {
            context_fields,
            context_inits,
            store_ctx_struct,
            store_ctx_data,
            link_interfaces,
            server_starts,
            view_impls,
        })
    }
}

pub fn expand(pre_expand: PreExpand) -> TokenStream {
    let PreExpand {
        context_fields,
        context_inits,
        store_ctx_struct,
        store_ctx_data,
        link_interfaces,
        server_starts,
        view_impls,
    } = pre_expand;

    quote! {
        use anyhow::Context as _;
        use futures::future::{BoxFuture, try_join_all};
        use runtime::{Resource, Server};
        use wasmtime_wasi::{WasiCtxBuilder, ResourceTable};
        use wasmtime::component::InstancePre;

        /// Run the specified wasm guest using the configured runtime.
        pub async fn runtime_run(wasm: std::path::PathBuf) -> anyhow::Result<()> {
            runtime_cli::RuntimeConfig::from_wasm(&wasm)?;

            let mut compiled = runtime::Runtime::new()
                .build(&wasm)
                .with_context(|| format!("compiling {}", wasm.display()))?;
            let run_state = Context::new(&mut compiled)
                .await
                .context("preparing runtime state")?;
            run_state.start().await.context("starting runtime services")
        }

        /// Runtime state holding pre-instantiated components and backend connections.
        #[derive(Clone)]
        struct Context {
            instance_pre: InstancePre<StoreCtx>,
            #(#context_fields,)*
        }

        impl Context {
            /// Creates a new runtime state by linking WASI interfaces and connecting to backends.
            async fn new(compiled: &mut runtime::Compiled<StoreCtx>) -> anyhow::Result<Self> {
                // Link all enabled WASI interfaces to the component
                #(#link_interfaces)*

                Ok(Self {
                    instance_pre: compiled.pre_instantiate()?,
                    #(#context_inits,)*
                })
            }

            /// Starts all enabled server interfaces (HTTP, messaging, websockets).
            async fn start(&self) -> anyhow::Result<()> {
                let futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = vec![
                    #(#server_starts,)*
                ];
                try_join_all(futures).await?;
                Ok(())
            }
        }

        impl runtime::State for Context {
            type StoreCtx = StoreCtx;

            fn instance_pre(&self) -> &InstancePre<Self::StoreCtx> {
                &self.instance_pre
            }

            fn new_store(&self) -> Self::StoreCtx {
                let wasi_ctx = WasiCtxBuilder::new()
                    .inherit_args()
                    .inherit_env()
                    .inherit_stdin()
                    .stdout(tokio::io::stdout())
                    .stderr(tokio::io::stderr())
                    .build();

                StoreCtx {
                    table: ResourceTable::new(),
                    wasi: wasi_ctx,
                    #(#store_ctx_data,)*
                }
            }
        }

        /// Per-instance data shared between the WebAssembly runtime and host functions.
        pub struct StoreCtx {
            pub table: wasmtime_wasi::ResourceTable,
            pub wasi: wasmtime_wasi::WasiCtx,
            #(#store_ctx_struct,)*
        }

        // WASI View Implementations
        impl wasmtime_wasi::WasiView for StoreCtx {
            fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
                wasmtime_wasi::WasiCtxView {
                    ctx: &mut self.wasi,
                    table: &mut self.table,
                }
            }
        }

        #(#view_impls)*
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
