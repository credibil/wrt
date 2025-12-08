#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use wasi_blobstore::{WasiBlobstore, WasiBlobstoreCtxImpl as BlobstoreDefault};
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};
mod runtime {
    use super::*;
    use kernel::anyhow::Context as _;
    use kernel::futures::future::{BoxFuture, try_join_all};
    use kernel::{Backend, Server};
    use kernel::wasmtime::component::InstancePre;
    use kernel::wasmtime_wasi::{WasiCtxBuilder, ResourceTable};
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
    /// Initiator state holding pre-instantiated components and backend
    /// connections.
    struct Context {
        instance_pre: InstancePre<StoreCtx>,
        pub httpdefault: HttpDefault,
        pub oteldefault: OtelDefault,
        pub blobstoredefault: BlobstoreDefault,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Context {
        #[inline]
        fn clone(&self) -> Context {
            Context {
                instance_pre: ::core::clone::Clone::clone(&self.instance_pre),
                httpdefault: ::core::clone::Clone::clone(&self.httpdefault),
                oteldefault: ::core::clone::Clone::clone(&self.oteldefault),
                blobstoredefault: ::core::clone::Clone::clone(&self.blobstoredefault),
            }
        }
    }
    impl Context {
        /// Creates a new runtime state by linking WASI interfaces and
        /// connecting to backends.
        async fn new(compiled: &mut kernel::Compiled<StoreCtx>) -> anyhow::Result<Self> {
            compiled.link(WasiHttp)?;
            compiled.link(WasiOtel)?;
            compiled.link(WasiBlobstore)?;
            Ok(Self {
                instance_pre: compiled.pre_instantiate()?,
                httpdefault: HttpDefault::connect().await?,
                oteldefault: OtelDefault::connect().await?,
                blobstoredefault: BlobstoreDefault::connect().await?,
            })
        }
        /// Start servers
        /// N.B. for simplicity, all hosts are "servers" with a default
        /// implementation the does nothing
        async fn start(&self) -> anyhow::Result<()> {
            let futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = <[_]>::into_vec(
                ::alloc::boxed::box_new([
                    Box::pin(WasiHttp.run(self)),
                    Box::pin(WasiOtel.run(self)),
                    Box::pin(WasiBlobstore.run(self)),
                ]),
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
                wasi_http: self.httpdefault.clone(),
                wasi_otel: self.oteldefault.clone(),
                wasi_blobstore: self.blobstoredefault.clone(),
            }
        }
    }
    /// Per-guest instance data shared between the runtime and guest
    pub struct StoreCtx {
        pub table: wasmtime_wasi::ResourceTable,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_http: HttpDefault,
        pub wasi_otel: OtelDefault,
        pub wasi_blobstore: BlobstoreDefault,
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
    impl wasi_blobstore::WasiBlobstoreView for StoreCtx {
        fn blobstore(&mut self) -> wasi_blobstore::WasiBlobstoreCtxView<'_> {
            wasi_blobstore::WasiBlobstoreCtxView {
                ctx: &mut self.wasi_blobstore,
                table: &mut self.table,
            }
        }
    }
}
use kernel::tokio;
fn main() -> anyhow::Result<()> {
    let body = async {
        use kernel::Parser;
        match kernel::Cli::parse().command {
            kernel::Command::Run { wasm } => runtime::run(wasm).await,
            _ => ::core::panicking::panic("internal error: entered unreachable code"),
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
