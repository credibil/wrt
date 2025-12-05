mod default_impl;
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
use std::marker::PhantomData;

use kernel::{FutureResult, Host, WasiHostCtx, WasiHostCtxView, WasiHostView};
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use wasmtime::component::{HasData, Linker};

pub use self::default_impl::WasiOtelCtxImpl;
use self::generated::wasi::otel as wasi;

#[derive(Debug, Default)]
pub struct WasiOtel<T: WasiOtelCtx> {
    _priv: PhantomData<fn() -> T>,
}

impl<T, C> Host<T> for WasiOtel<C>
where
    C: WasiOtelCtx,
    T: WasiHostView<C> + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        wasi::tracing::add_to_linker::<_, Self>(linker, T::ctx_view)?;
        wasi::metrics::add_to_linker::<_, Self>(linker, T::ctx_view)?;
        wasi::types::add_to_linker::<_, Self>(linker, T::ctx_view)?;
        wasi::resource::add_to_linker::<_, Self>(linker, T::ctx_view)
    }
}

impl<C: WasiOtelCtx + 'static> HasData for WasiOtel<C> {
    type Data<'a> = WasiHostCtxView<'a, C>;
}

/// A trait which provides internal WASI OpenTelemetry context.
///
/// This is implemented by the resource-specific provider of OpenTelemetry
/// functionality.
pub trait WasiOtelCtx: WasiHostCtx {
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
