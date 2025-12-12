use anyhow::{Context, Result};
use bytes::Bytes;
use wasmtime::component::{Access, Resource};
use wasmtime_wasi::p2::bindings::io::streams::{InputStream, OutputStream};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use crate::host::generated::wasi::blobstore::types::{
    Host, HostIncomingValue, HostIncomingValueWithStore, HostOutgoingValue,
    HostOutgoingValueWithStore, IncomingValueSyncBody,
};
use crate::host::{WasiBlobstore, WasiBlobstoreCtxView};

pub type IncomingValue = Bytes;
pub type OutgoingValue = MemoryOutputPipe;

impl HostIncomingValueWithStore for WasiBlobstore {
    fn incoming_value_consume_sync<T>(
        mut host: Access<'_, T, Self>, this: Resource<IncomingValue>,
    ) -> Result<IncomingValueSyncBody> {
        let value = host.get().table.get(&this).context("IncomingValue not found")?;
        Ok(value.to_vec())
    }

    fn incoming_value_consume_async<T>(
        mut host: Access<'_, T, Self>, this: Resource<IncomingValue>,
    ) -> Result<Resource<InputStream>> {
        let value = host.get().table.get(&this).context("IncomingValue not found")?;
        let rs = MemoryInputPipe::new(value.clone());
        let stream: InputStream = Box::new(rs);
        Ok(host.get().table.push(stream)?)
    }

    fn size<T>(mut host: Access<'_, T, Self>, self_: Resource<IncomingValue>) -> Result<u64> {
        let value = host.get().table.get(&self_).context("IncomingValue not found")?;
        Ok(value.len() as u64)
    }

    fn drop<T>(mut accessor: Access<'_, T, Self>, rep: Resource<IncomingValue>) -> Result<()> {
        Ok(accessor.get().table.delete(rep).map(|_| ())?)
    }
}

impl HostOutgoingValueWithStore for WasiBlobstore {
    fn new_outgoing_value<T>(mut host: Access<'_, T, Self>) -> Result<Resource<OutgoingValue>> {
        Ok(host.get().table.push(OutgoingValue::new(1024))?)
    }

    fn outgoing_value_write_body<T>(
        mut host: Access<'_, T, Self>, self_: Resource<OutgoingValue>,
    ) -> Result<Result<Resource<OutputStream>, ()>> {
        let value = host.get().table.get(&self_).context("OutgoingValue not found")?;
        let stream: OutputStream = Box::new(value.clone());
        Ok(host.get().table.push(stream).map_err(|_e| ()))
    }

    fn finish<T>(_: Access<'_, T, Self>, _self_: Resource<OutgoingValue>) -> Result<()> {
        Ok(())
    }

    fn drop<T>(mut accessor: Access<'_, T, Self>, rep: Resource<OutgoingValue>) -> Result<()> {
        Ok(accessor.get().table.delete(rep).map(|_| ())?)
    }
}

impl Host for WasiBlobstoreCtxView<'_> {
    fn convert_error(&mut self, err: anyhow::Error) -> Result<String> {
        Ok(err.to_string())
    }
}
impl HostIncomingValue for WasiBlobstoreCtxView<'_> {}
impl HostOutgoingValue for WasiBlobstoreCtxView<'_> {}
