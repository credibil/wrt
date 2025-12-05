use anyhow::Result;
use opentelemetry_sdk::Resource;
use runtime::WasiHostCtxView;
use wasmtime::component::Accessor;

use crate::host::generated::wasi::otel::{resource, types};
use crate::host::{WasiOtel, WasiOtelCtx};

impl<C: WasiOtelCtx> resource::HostWithStore for WasiOtel<C> {
    async fn resource<T>(_: &Accessor<T, Self>) -> Result<types::Resource> {
        let Some(resource) = credibil_otel::init::resource() else {
            ::tracing::warn!("otel resource not initialized");
            let empty = &Resource::builder().build();
            return Ok(empty.into());
        };
        Ok(resource.into())
    }
}

impl<C: WasiOtelCtx> resource::Host for WasiHostCtxView<'_, C> {}
