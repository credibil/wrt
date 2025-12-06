#![feature(prelude_import)]
//! WRT runtime CLI entry point.
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use anyhow::Result;
use kernel::{Cli, Command, Parser};
use wasi_http::{WasiHttpCtxImpl, WasiHttp};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl};
mod runtime {
    use super::*;
    use anyhow::Context as _;
    use futures::future::{BoxFuture, try_join_all};
    use kernel::{Backend, Server, WasiHostCtxView, WasiHostView};
    use wasmtime_wasi::{WasiCtxBuilder, ResourceTable};
    use wasmtime::component::InstancePre;
    /// Run the specified wasm guest using the configured runtime.
    pub async fn run(wasm: std::path::PathBuf) -> anyhow::Result<()> {
        let mut compiled = kernel::create(&wasm)
            .with_context(|| ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("compiling {0}", wasm.display()))
            }))?;
        let run_state = Context::new(&mut compiled)
            .await
            .context("preparing runtime state")?;
        run_state.start().await.context("starting runtime services")
    }
    /// Initiator state holding pre-instantiated components and backend connections.
    struct Context {
        instance_pre: InstancePre<StoreCtx>,
        pub wasiotelctximpl: WasiOtelCtxImpl,
        pub wasihttpctximpl: WasiHttpCtxImpl,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Context {
        #[inline]
        fn clone(&self) -> Context {
            Context {
                instance_pre: ::core::clone::Clone::clone(&self.instance_pre),
                wasiotelctximpl: ::core::clone::Clone::clone(&self.wasiotelctximpl),
                wasihttpctximpl: ::core::clone::Clone::clone(&self.wasihttpctximpl),
            }
        }
    }
    impl Context {
        /// Creates a new runtime state by linking WASI interfaces and connecting to backends.
        async fn new(compiled: &mut kernel::Compiled<StoreCtx>) -> anyhow::Result<Self> {
            compiled.link(WasiHttp)?;
            compiled.link(WasiOtel)?;
            Ok(Self {
                instance_pre: compiled.pre_instantiate()?,
                wasiotelctximpl: WasiOtelCtxImpl::connect().await?,
                wasihttpctximpl: WasiHttpCtxImpl::connect().await?,
            })
        }
        /// start enabled servers
        async fn start(&self) -> anyhow::Result<()> {
            let futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = <[_]>::into_vec(
                ::alloc::boxed::box_new([Box::pin(WasiHttp.run(self))]),
            );
            try_join_all(futures).await?;
            Ok(())
        }
    }
    impl kernel::State for Context {
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
                wasi_http: self.wasihttpctximpl.clone(),
                wasi_otel: self.wasiotelctximpl.clone(),
            }
        }
    }
    /// Per-guest instance data shared between the runtime and guest
    pub struct StoreCtx {
        pub table: wasmtime_wasi::ResourceTable,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_http: WasiHttpCtxImpl,
        pub wasi_otel: WasiOtelCtxImpl,
    }
    impl wasmtime_wasi::WasiView for StoreCtx {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.table,
            }
        }
    }
    impl wasi_http::WasiHttpView for StoreCtx {
        fn http(&mut self) -> wasi_http::WasiHttpCtxView<'_> {
            wasi_http::WasiHttpCtxView {
                ctx: &mut self.wasi_http,
                table: &mut self.table,
            }
        }
    }
    impl wasi_otel::WasiOtelView for StoreCtx {
        fn otel(&mut self) -> wasi_otel::WasiOtelCtxView<'_> {
            wasi_otel::WasiOtelCtxView {
                ctx: &mut self.wasi_otel,
                table: &mut self.table,
            }
        }
    }
}
fn main() -> Result<()> {
    let body = async {
        match Cli::parse().command {
            Command::Run { wasm } => runtime::run(wasm).await,
            Command::Compile { wasm, output } => kernel::compile(&wasm, output),
        }
    };
    #[allow(
        clippy::expect_used,
        clippy::diverging_sub_expression,
        clippy::needless_return,
        clippy::unwrap_in_result
    )]
    {
        return tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(body);
    }
}
