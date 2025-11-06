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
            "wasi:otel/types/error" => Error,
        }
    });
}

use std::fmt::Debug;

use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use runtime::Host;
use wasmtime::component::{HasData, Linker, ResourceTable};

use self::generated::wasi::otel as wasi;

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

const DEF_HTTP_URL: &str = "http://localhost:4318";

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
    fn export(&self, request: http::Request<Vec<u8>>) -> FutureResult<()> {
        async move {
            let (parts, body) = request.into_parts();

            if let Err(e) = reqwest::Client::new()
                .post(parts.uri.to_string())
                .headers(parts.headers)
                .body(body)
                .send()
                .await
            {
                tracing::error!("failed to send traces: {e}");
            }

            Ok(())
        }
        .boxed()
    }
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
