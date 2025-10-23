use wasmtime::component::Resource;

use crate::host::generated::wasi::messaging::types;
pub use crate::host::generated::wasi::messaging::types::Error;
use crate::host::resource::ClientProxy;
use crate::host::{Host, Result};

impl types::Host for Host<'_> {
    fn convert_error(&mut self, err: Error) -> anyhow::Result<Error> {
        Ok(err)
    }
}

impl types::HostClient for Host<'_> {
    async fn connect(&mut self, name: String) -> Result<Resource<ClientProxy>> {
        tracing::trace!("HostClient::connect {name}");
        let client = ClientProxy::try_from(&name)?;
        let resource = self.table.push(client)?;
        Ok(resource)
    }

    async fn disconnect(&mut self, _rep: Resource<ClientProxy>) -> Result<()> {
        tracing::trace!("HostClient::disconnect");
        Ok(())
    }

    async fn drop(&mut self, rep: Resource<ClientProxy>) -> anyhow::Result<()> {
        tracing::trace!("HostClient::drop");
        self.table.delete(rep)?;
        Ok(())
    }
}
