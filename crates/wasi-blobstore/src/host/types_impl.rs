use anyhow::{Result, anyhow};
use bytes::Bytes;
use wasmtime::component::{Accessor, Resource};
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
    async fn incoming_value_consume_sync<T>(
        accessor: &Accessor<T, Self>, this: Resource<IncomingValue>,
    ) -> Result<IncomingValueSyncBody> {
        let value = get_incoming(accessor, &this)?;
        Ok(value.to_vec())
    }

    async fn incoming_value_consume_async<T>(
        accessor: &Accessor<T, Self>, this: Resource<IncomingValue>,
    ) -> Result<Resource<InputStream>> {
        let value = get_incoming(accessor, &this)?;
        let rs = MemoryInputPipe::new(value);
        let stream: InputStream = Box::new(rs);
        Ok(accessor.with(|mut store| store.get().table.push(stream))?)
    }

    async fn size<T>(accessor: &Accessor<T, Self>, self_: Resource<IncomingValue>) -> Result<u64> {
        let value = get_incoming(accessor, &self_)?;
        Ok(value.len() as u64)
    }

    async fn drop<T>(accessor: &Accessor<T, Self>, rep: Resource<IncomingValue>) -> Result<()> {
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

impl HostOutgoingValueWithStore for WasiBlobstore {
    async fn new_outgoing_value<T>(
        accessor: &Accessor<T, Self>,
    ) -> Result<Resource<OutgoingValue>> {
        accessor.with(|mut store| Ok(store.get().table.push(OutgoingValue::new(1024))?))
    }

    async fn outgoing_value_write_body<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OutgoingValue>,
    ) -> Result<Result<Resource<OutputStream>, ()>> {
        let value = get_outgoing(accessor, &self_)?;
        let stream: OutputStream = Box::new(value);
        Ok(accessor.with(|mut store| store.get().table.push(stream).map_err(|_e| ())))
    }

    async fn finish<T>(_: &Accessor<T, Self>, _self_: Resource<OutgoingValue>) -> Result<()> {
        Ok(())
    }

    async fn drop<T>(accessor: &Accessor<T, Self>, rep: Resource<OutgoingValue>) -> Result<()> {
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

impl Host for WasiBlobstoreCtxView<'_> {
    fn convert_error(&mut self, err: anyhow::Error) -> Result<String> {
        Ok(err.to_string())
    }
}
impl HostIncomingValue for WasiBlobstoreCtxView<'_> {}
impl HostOutgoingValue for WasiBlobstoreCtxView<'_> {}

pub fn get_incoming<T>(
    accessor: &Accessor<T, WasiBlobstore>, self_: &Resource<IncomingValue>,
) -> Result<IncomingValue> {
    accessor.with(|mut store| {
        let incoming =
            store.get().table.get(self_).map_err(|e| anyhow!("IncomingValue not found: {e}"))?;
        Ok(incoming.clone())
    })
}

pub fn get_outgoing<T>(
    accessor: &Accessor<T, WasiBlobstore>, self_: &Resource<OutgoingValue>,
) -> Result<OutgoingValue> {
    accessor.with(|mut store| {
        let outgoing =
            store.get().table.get(self_).map_err(|e| anyhow!("OutgoingValue not found: {e}"))?;
        Ok(outgoing.clone())
    })
}
