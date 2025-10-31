use anyhow::Result;
use wasmtime::component::Accessor;

use crate::host::generated::wasi::otel::{resource, types};
use crate::host::{WasiOtel, WasiOtelCtxView};

impl resource::Host for WasiOtelCtxView<'_> {}

impl resource::HostWithStore for WasiOtel {
    async fn resource<T>(_: &Accessor<T, Self>) -> Result<types::Resource> {
        // let Some(resource) = init::resource() else {
        //     ::tracing::warn!("otel resource not initialized");
        //     let empty = &Resource::builder().build();
        //     return Ok(empty.into());
        // };
        // Ok(resource.into())
        unimplemented!()
    }
}
