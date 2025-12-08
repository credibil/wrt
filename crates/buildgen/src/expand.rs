//! # Generated Code Expansion
//!
//! Expands the generated code into a complete runtime implementation.

use proc_macro2::TokenStream;
use quote::quote;

use crate::generate::Generated;

pub fn expand(generated: Generated) -> TokenStream {
    let Generated {
        main_fn,
        context_fields,
        store_ctx_fields,
        store_ctx_values,
        host_trait_impls,
        server_trait_impls,
        wasi_view_impls,
    } = generated;



    quote! {
        mod runtime {
            use super::*;

            use anyhow::Context as _;
            use futures::future::{BoxFuture, try_join_all};
            use kernel::{Backend, Server};
            use wasmtime_wasi::{WasiCtxBuilder, ResourceTable};
            use wasmtime::component::InstancePre;

            /// Run the specified wasm guest using the configured runtime.
            pub async fn run(wasm: std::path::PathBuf) -> anyhow::Result<()> {
                let mut compiled = kernel::create(&wasm)
                    .with_context(|| format!("compiling {}", wasm.display()))?;
                let run_state = Context::new(&mut compiled)
                    .await
                    .context("preparing runtime state")?;
                run_state.start().await.context("starting runtime services")
            }

            /// Initiator state holding pre-instantiated components and backend
            /// connections.
            #[derive(Clone)]
            struct Context {
                instance_pre: InstancePre<StoreCtx>,
                #(pub #context_fields,)*
            }

            impl Context {
                /// Creates a new runtime state by linking WASI interfaces and
                /// connecting to backends.
                async fn new(compiled: &mut kernel::Compiled<StoreCtx>) -> anyhow::Result<Self> {
                    // link enabled WASI components
                    #(compiled.link(#host_trait_impls)?;)*

                    Ok(Self {
                        instance_pre: compiled.pre_instantiate()?,
                        #(#context_fields::connect().await?,)*
                    })
                }

                /// Start servers
                /// N.B. for simplicity, all hosts are "servers" with a default
                /// implementation the does nothing
                async fn start(&self) -> anyhow::Result<()> {
                    let futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = vec![
                        #(Box::pin(#server_trait_impls.run(self)),)*
                    ];
                    try_join_all(futures).await?;
                    Ok(())
                }
            }

            impl kernel::State for Context {
                type StoreCtx = StoreCtx;

                fn instance_pre(&self) -> &InstancePre<Self::StoreCtx> {
                    &self.instance_pre
                }

                fn store(&self) -> Self::StoreCtx {
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
                        #(#store_ctx_values,)*
                    }
                }
            }

            /// Per-guest instance data shared between the runtime and guest
            pub struct StoreCtx {
                pub table: wasmtime_wasi::ResourceTable,
                pub wasi: wasmtime_wasi::WasiCtx,
                #(pub #store_ctx_fields,)*
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

            #(#wasi_view_impls)*
        }
        #main_fn
    }
}
