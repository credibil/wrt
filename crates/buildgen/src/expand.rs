use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

use crate::parse::BuildInput;

pub struct PreExpand {
    context_fields: Vec<TokenStream>,
    context_inits: Vec<TokenStream>,
    store_ctx_fields: Vec<TokenStream>,
    store_ctx_inits: Vec<TokenStream>,
    link_interfaces: Vec<TokenStream>,
    server_starts: Vec<TokenStream>,
    view_impls: Vec<TokenStream>,
}

impl TryFrom<BuildInput> for PreExpand {
    type Error = syn::Error;

    fn try_from(input: BuildInput) -> Result<Self, Self::Error> {
        // `RuntimeContext` struct fields & `new()` method initialization
        let mut context_fields = Vec::<TokenStream>::new();
        let mut context_inits = Vec::<TokenStream>::new();

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
        let mut store_ctx_fields = Vec::<TokenStream>::new();
        let mut store_ctx_inits = Vec::<TokenStream>::new();
        let mut link_interfaces = Vec::<TokenStream>::new();

        for host in &input.hosts {
            let field_ident = syn::parse_str::<Ident>(&host.name).unwrap();

            let backend = &host.backend;
            let field = quote! {
                pub #field_ident: #backend
            };
            let backend = field_name(&host.backend);
            let init = quote! {
                #field_ident: self.#backend.clone()
            };

            let host_ident = &host.ident;
            link_interfaces.push(quote! {
                compiled.link(#host_ident)?;
            });

            store_ctx_fields.push(field);
            store_ctx_inits.push(init);
        }

        // server start calls
        let server_starts: Vec<_> = input
            .hosts
            .iter()
            .filter(|host| host.is_server)
            .map(|host| {
                let host_ident = &host.ident;
                let module_name = snake_case(&host.name);
                let module = syn::parse_str::<syn::Ident>(&module_name).unwrap();

                quote! {
                    Box::pin(#module::#host_ident.run(self))
                }
            })
            .collect();

        // WasiViewXxx implementations
        let view_impls: Vec<_> = input
            .hosts
            .iter()
            .map(|host| {
                let view_trait = &host.view_trait;
                let view_method = &host.view_method;
                let ctx_view = &host.ctx_view;
                let field_name = syn::parse_str::<syn::Ident>(&host.name).unwrap();

                let module_name = snake_case(&host.name);
                let module = syn::parse_str::<syn::Ident>(&module_name).unwrap();

                quote! {
                    impl #module::#view_trait for StoreCtx {
                        fn #view_method(&mut self) -> #module::#ctx_view<'_> {
                            #module::#ctx_view {
                                ctx: &mut self.#field_name,
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
            store_ctx_fields,
            store_ctx_inits,
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
        store_ctx_fields,
        store_ctx_inits,
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
                    #(#store_ctx_inits,)*
                }
            }
        }

        /// Per-instance data shared between the WebAssembly runtime and host functions.
        pub struct StoreCtx {
            pub table: wasmtime_wasi::ResourceTable,
            pub wasi: wasmtime_wasi::WasiCtx,
            #(#store_ctx_fields,)*
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
fn field_name(field_type: &Type) -> syn::Ident {
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
