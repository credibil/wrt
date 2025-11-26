mod metrics_impl;
mod resource_impl;
mod tracing_impl;
mod types_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]
    pub use self::wasi::otel::types::Error;

    wasmtime::component::bindgen!({
        world: "otel",
        path: "wit",
        imports: {
            default: async | store | tracing | trappable,
        },
        trappable_error_type: {
            "wasi:otel/types.error" => Error,
        }
    });
}

use std::fmt::Debug;

use anyhow::{Context, Result, anyhow};
use fromenv::FromEnv;
use futures::FutureExt;
use futures::future::BoxFuture;
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use opentelemetry_proto::tonic::collector::metrics::v1::metrics_service_client::MetricsServiceClient;
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use opentelemetry_proto::tonic::collector::trace::v1::trace_service_client::TraceServiceClient;
use runtime::{Host, Resource};
use tonic::transport::Channel;
use tracing::instrument;
use wasmtime::component::{HasData, Linker, ResourceTable};

use self::generated::wasi::otel as wasi;

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

impl<T> Host<T> for WasiOtel
where
    T: WasiOtelView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        wasi::tracing::add_to_linker::<_, Self>(linker, T::otel)?;
        wasi::metrics::add_to_linker::<_, Self>(linker, T::otel)?;
        wasi::types::add_to_linker::<_, Self>(linker, T::otel)?;
        wasi::resource::add_to_linker::<_, Self>(linker, T::otel)
    }
}

#[derive(Debug)]
pub struct WasiOtel;
impl HasData for WasiOtel {
    type Data<'a> = WasiOtelCtxView<'a>;
}

/// A trait which provides internal WASI OpenTelemetry context.
///
/// This is implemented by the resource-specific provider of OpenTelemetry
/// functionality.
pub trait WasiOtelCtx: Debug + Send + Sync + 'static {
    /// Export traces using gRPC.
    ///
    /// Errors are logged but not propagated to prevent telemetry failures
    /// from affecting application logic.
    fn export_traces(&self, request: ExportTraceServiceRequest) -> FutureResult<()>;

    /// Export metrics using gRPC.
    ///
    /// Errors are logged but not propagated to prevent telemetry failures
    /// from affecting application logic.
    fn export_metrics(&self, request: ExportMetricsServiceRequest) -> FutureResult<()>;
}

/// View into [`WasiOtelCtx`] implementation and [`ResourceTable`].
pub struct WasiOtelCtxView<'a> {
    /// Mutable reference to the WASI OpenTelemetry context.
    pub ctx: &'a mut dyn WasiOtelCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

/// A trait which provides internal WASI OpenTelemetry state.
///
/// This is implemented by the `T` in `Linker<T>` — a single type shared across
/// all WASI components for the runtime build.
pub trait WasiOtelView: Send {
    /// Return a [`WasiOtelCtxView`] from mutable reference to self.
    fn otel(&mut self) -> WasiOtelCtxView<'_>;
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "OTEL_GRPC_URL", default = "http://localhost:4317")]
    pub grpc_url: String,
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}

#[derive(Debug, Clone)]
pub struct GrpcOtelCtx {
    traces_client: TraceServiceClient<Channel>,
    metrics_client: MetricsServiceClient<Channel>,
}

impl Resource for GrpcOtelCtx {
    type ConnectOptions = ConnectOptions;

    #[instrument(name = "GrpcOtel::connect_with")]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let options =
            ConnectOptions::from_env().finalize().context("loading connection options")?;
        tracing::debug!("connecting to OpenTelemetry gRPC endpoint at: {}", options.grpc_url);

        let traces_client = TraceServiceClient::connect(options.grpc_url.clone()).await?;
        let metrics_client = MetricsServiceClient::connect(options.grpc_url).await?;

        Ok(Self {
            traces_client,
            metrics_client,
        })
    }
}

impl WasiOtelCtx for GrpcOtelCtx {
    /// Export traces using gRPC.
    ///
    /// Errors are logged but not propagated to prevent telemetry failures
    /// from affecting application logic.
    fn export_traces(&self, request: ExportTraceServiceRequest) -> FutureResult<()> {
        let mut client = self.traces_client.clone();

        async move {
            if let Err(e) = client.export(request).await {
                tracing::error!("failed to send traces via gRPC: {e}");
            }
            Ok(())
        }
        .boxed()
    }

    /// Export metrics using gRPC.
    ///
    /// Errors are logged but not propagated to prevent telemetry failures
    /// from affecting application logic.
    fn export_metrics(&self, request: ExportMetricsServiceRequest) -> FutureResult<()> {
        let mut client = self.metrics_client.clone();

        async move {
            if let Err(e) = client.export(request).await {
                tracing::error!("failed to send metrics via gRPC: {e}");
            }
            Ok(())
        }
        .boxed()
    }
}
