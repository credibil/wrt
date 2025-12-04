use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Type};

use crate::parse::BuildInput;

pub struct PreExpand {
    context: Vec<TokenStream>,
    server_starts: Vec<TokenStream>,
    view_impls: Vec<TokenStream>,
    host_types: Vec<Type>,
    host_idents: Vec<Ident>,
    backend_types: Vec<Type>,
    backend_idents: Vec<Ident>,
}

impl TryFrom<BuildInput> for PreExpand {
    type Error = syn::Error;

    fn try_from(input: BuildInput) -> Result<Self, Self::Error> {
        // `Context` struct
        let mut context = Vec::new();
        for backend in input.backends {
            let field = field_name(&backend);
            context.push(quote! {#field: #backend});
        }

        // let mut link_calls = Vec::new();
        let mut server_starts = Vec::new();
        let mut view_impls = Vec::new();
        let mut host_types = Vec::new();
        let mut host_idents = Vec::new();
        let mut backend_types = Vec::new();
        let mut backend_idents = Vec::new();

        for host in &input.hosts {
            let host_type = &host.host_type;
            let host_name = quote! {#host_type}.to_string();
            let host_ident = syn::parse_str::<Ident>(&snake_case(&host_name))?;

            host_types.push(host_type.clone());
            host_idents.push(host_ident.clone());
            backend_types.push(host.backend.clone());
            backend_idents.push(field_name(&host.backend));

            let module = &host_ident;

            // servers
            if host.is_server {
                let start = quote! {Box::pin(#module::#host_type.run(self))};
                server_starts.push(start);
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
            view_impls.push(view);
        }

        Ok(Self {
            context,
            server_starts,
            view_impls,
            host_types,
            host_idents,
            backend_types,
            backend_idents,
        })
    }
}

pub fn expand(pre_expand: PreExpand) -> TokenStream {
    let PreExpand {
        context,
        server_starts,
        view_impls,
        host_types,
        host_idents,
        backend_types,
        backend_idents,
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
            #(#context,)*
        }

        impl Context {
            /// Creates a new runtime state by linking WASI interfaces and connecting to backends.
            async fn new(compiled: &mut runtime::Compiled<StoreCtx>) -> anyhow::Result<Self> {
                // link enabled WASI components
                #(compiled.link(#host_types)?;)*

                Ok(Self {
                    instance_pre: compiled.pre_instantiate()?,
                    #(#context::connect().await?,)*
                })
            }

            /// start enabled servers
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
                    #(#host_idents: self.#backend_idents.clone(),)*
                }
            }
        }

        /// Per-guest instance data shared between the runtime and guest
        pub struct StoreCtx {
            pub table: wasmtime_wasi::ResourceTable,
            pub wasi: wasmtime_wasi::WasiCtx,
            #(pub #host_idents: #backend_types,)*
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
