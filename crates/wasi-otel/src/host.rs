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

use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use runtime::Host;
use tonic::transport::Channel;
use wasmtime::component::{HasData, Linker, ResourceTable};

use self::generated::wasi::otel as wasi;

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

const DEF_GRPC_ENDPOINT: &str = "http://localhost:4317";

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
    fn export_traces(
        &self, request: opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest,
    ) -> FutureResult<()> {
        async move {
            use opentelemetry_proto::tonic::collector::trace::v1::trace_service_client::TraceServiceClient;

            match connect_grpc().await {
                Ok(channel) => {
                    let mut client = TraceServiceClient::new(channel);
                    if let Err(e) = client.export(request).await {
                        tracing::error!("failed to send traces via gRPC: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("failed to connect to gRPC endpoint: {e}");
                }
            }

            Ok(())
        }
        .boxed()
    }

    /// Export metrics using gRPC.
    ///
    /// Errors are logged but not propagated to prevent telemetry failures
    /// from affecting application logic.
    fn export_metrics(
        &self,
        request: opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest,
    ) -> FutureResult<()> {
        async move {
            use opentelemetry_proto::tonic::collector::metrics::v1::metrics_service_client::MetricsServiceClient;

            match connect_grpc().await {
                Ok(channel) => {
                    let mut client = MetricsServiceClient::new(channel);
                    if let Err(e) = client.export(request).await {
                        tracing::error!("failed to send metrics via gRPC: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("failed to connect to gRPC endpoint: {e}");
                }
            }

            Ok(())
        }
        .boxed()
    }
}

/// Connect to the OpenTelemetry gRPC endpoint.
///
/// Reads the endpoint from `OTEL_GRPC_URL` environment variable or uses default.
///
/// Note: In tonic, `Channel` is a cheaply-cloneable handle that manages an
/// internal connection pool. However, `connect()` may still attempt to establish
/// an initial connection. For better performance in production, consider
/// implementing a shared channel that is created once and reused across exports.
/// The current implementation prioritizes simplicity and correctness.
async fn connect_grpc() -> Result<Channel> {
    let endpoint =
        std::env::var("OTEL_GRPC_URL").unwrap_or_else(|_| DEF_GRPC_ENDPOINT.to_string());

    let channel = Channel::from_shared(endpoint.clone())
        .map_err(|e| anyhow::anyhow!("invalid gRPC endpoint {endpoint}: {e}"))?;

    channel
        .connect()
        .await
        .map_err(|e| anyhow::anyhow!("failed to connect to gRPC endpoint {endpoint}: {e}"))
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

#[derive(Debug)]
pub struct DefaultOtelCtx;
impl WasiOtelCtx for DefaultOtelCtx {}
