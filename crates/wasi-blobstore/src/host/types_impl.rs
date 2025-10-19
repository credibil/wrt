use anyhow::Result;
use bytes::Bytes;
use wasmtime::component::Resource;
use wasmtime_wasi::p2::bindings::io::streams::{InputStream, OutputStream};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use crate::host::Host;
use crate::host::generated::wasi::blobstore::types::{self, IncomingValueSyncBody};

pub type IncomingValue = Bytes;
pub type OutgoingValue = MemoryOutputPipe;

impl types::Host for Host<'_> {
    fn convert_error(&mut self, err: anyhow::Error) -> Result<String> {
        tracing::error!("{err}");
        Ok(err.to_string())
    }
}

impl types::HostIncomingValue for Host<'_> {
    async fn incoming_value_consume_sync(
        &mut self, this: Resource<IncomingValue>,
    ) -> Result<IncomingValueSyncBody> {
        let value = self.table.get(&this)?;
        Ok(value.to_vec())
    }

    async fn incoming_value_consume_async(
        &mut self, this: Resource<IncomingValue>,
    ) -> Result<Resource<InputStream>> {
        let value = self.table.get(&this)?;
        let rs = MemoryInputPipe::new(value.clone());
        let stream: InputStream = Box::new(rs);

        Ok(self.table.push(stream)?)
    }

    async fn size(&mut self, self_: Resource<IncomingValue>) -> Result<u64> {
        let value = self.table.get(&self_)?;
        Ok(value.len() as u64)
    }

    async fn drop(&mut self, rep: Resource<IncomingValue>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}

impl types::HostOutgoingValue for Host<'_> {
    async fn new_outgoing_value(&mut self) -> Result<Resource<OutgoingValue>> {
        Ok(self.table.push(OutgoingValue::new(1024))?)
    }

    async fn outgoing_value_write_body(
        &mut self, self_: Resource<OutgoingValue>,
    ) -> Result<Result<Resource<OutputStream>, ()>> {
        let value = self.table.get(&self_)?;
        let stream: OutputStream = Box::new(value.clone());
        Ok(self.table.push(stream).map_err(|_e| ()))
    }

    async fn finish(&mut self, _self_: Resource<OutgoingValue>) -> Result<()> {
        // self.table.delete(data)?;
        Ok(())
    }

    async fn drop(&mut self, rep: Resource<OutgoingValue>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}
