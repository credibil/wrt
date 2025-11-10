use wasmtime::component::{Accessor, Resource};

use crate::host::generated::wasi::messaging::types;
pub use crate::host::generated::wasi::messaging::types::{
    Error, Host, HostClient, HostClientWithStore, HostMessage, HostMessageWithStore, Topic,
};
use crate::host::resource::{ClientProxy, MessageProxy};
use crate::host::{Result, WasiMessaging, WasiMessagingCtxView};

impl HostClientWithStore for WasiMessaging {
    async fn connect<T>(
        accessor: &Accessor<T, Self>, _name: String,
    ) -> Result<Resource<ClientProxy>> {
        let client = accessor.with(|mut store| store.get().ctx.connect()).await?;
        let proxy = ClientProxy(client);
        Ok(accessor.with(|mut store| store.get().table.push(proxy))?)
    }

    async fn disconnect<T>(_: &Accessor<T, Self>, _rep: Resource<ClientProxy>) -> Result<()> {
        Ok(())
    }

    async fn drop<T>(
        accessor: &Accessor<T, Self>, rep: Resource<ClientProxy>,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

impl HostMessageWithStore for WasiMessaging {
    /// Create a new message with the given payload.
    async fn new<T>(
        accessor: &Accessor<T, Self>, data: Vec<u8>,
    ) -> anyhow::Result<Resource<MessageProxy>> {
        let message = accessor.with(|mut store| store.get().ctx.new_message(data)).await?;
        let proxy = MessageProxy(message);
        Ok(accessor.with(|mut store| store.get().table.push(proxy))?)
    }

    /// The topic/subject/channel this message was received on, if any.
    async fn topic<T>(
        accessor: &Accessor<T, Self>, self_: Resource<MessageProxy>,
    ) -> anyhow::Result<Option<Topic>> {
        let message = get_message(accessor, &self_)?;
        let topic = message.topic();
        if topic.is_empty() { Ok(None) } else { Ok(Some(topic)) }
    }

    /// An optional content-type describing the format of the data in the
    /// message. This is sometimes described as the "format" type".
    async fn content_type<T>(
        accessor: &Accessor<T, Self>, self_: Resource<MessageProxy>,
    ) -> anyhow::Result<Option<String>> {
        let message = get_message(accessor, &self_)?;
        if let Some (md) = message.metadata() {
            if let Some(content_type) = md.get("content-type") {
                return Ok(Some(content_type.clone()));
            }
            return Ok(None);
        }
        Ok(None)
    }

    /// Set the content-type describing the format of the data in the message.
    /// This is sometimes described as the "format" type.
    async fn set_content_type<T>(
        accessor: &Accessor<T, Self>, self_: Resource<MessageProxy>, content_type: String,
    ) -> anyhow::Result<()> {
        let message = get_message(accessor, &self_)?;
        let updated_message = accessor
            .with(|mut store| store.get().ctx.set_content_type(message.0, content_type))
            .await?;
        accessor.with(|mut store| store.get().table.push(updated_message))?;
        Ok(())
    }

    /// An opaque blob of data.
    async fn data<T>(
        accessor: &Accessor<T, Self>, self_: Resource<MessageProxy>,
    ) -> anyhow::Result<Vec<u8>> {
        let message = get_message(accessor, &self_)?;
        Ok(message.payload())
    }

    /// Set the opaque blob of data for this message, discarding the old value.
    async fn set_data<T>(
        accessor: &Accessor<T, Self>, self_: Resource<MessageProxy>, data: Vec<u8>,
    ) -> anyhow::Result<()> {
        let message = get_message(accessor, &self_)?;
        let updated_message = accessor.with(|mut store| store.get().ctx.set_payload(message.0, data)).await?;
        accessor.with(|mut store| store.get().table.push(updated_message))?;
        Ok(())
    }

    /// Get the metadata associated with this message.    
    async fn metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<MessageProxy>,
    ) -> anyhow::Result<Option<types::Metadata>> {
        let message = get_message(accessor, &self_)?;
        let md = message.metadata().map(std::convert::Into::into);
        Ok(md)
    }

    /// Append a key-value pair to the metadata of this message.
    async fn add_metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<MessageProxy>, key: String, value: String,
    ) -> anyhow::Result<()> {
        let message = get_message(accessor, &self_)?;
        let updated_message = accessor
            .with(|mut store| store.get().ctx.add_metadata(message.0, key, value))
            .await?;
        accessor.with(|mut store| store.get().table.push(updated_message))?;
        Ok(())
    }

    /// Set all the metadata on this message, replacing any existing metadata.
    async fn set_metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<MessageProxy>, meta: types::Metadata,
    ) -> anyhow::Result<()> {
        let message = get_message(accessor, &self_)?;
        let updated_message = accessor
            .with(|mut store| store.get().ctx.set_metadata(message.0, meta.into()))
            .await?;
        accessor.with(|mut store| store.get().table.push(updated_message))?;
        Ok(())
    }

    /// Remove a key-value pair from the metadata of a message.
    async fn remove_metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<MessageProxy>, key: String,
    ) -> anyhow::Result<()> {
        let message = get_message(accessor, &self_)?;
        let updated_message = accessor
            .with(|mut store| store.get().ctx.remove_metadata(message.0, key))
            .await?;
        accessor.with(|mut store| store.get().table.push(updated_message))?;
        Ok(())
    }

    async fn drop<T>(
        accessor: &Accessor<T, Self>, rep: Resource<MessageProxy>,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

impl Host for WasiMessagingCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> anyhow::Result<Error> {
        Ok(err)
    }
}
impl HostClient for WasiMessagingCtxView<'_> {}
impl HostMessage for WasiMessagingCtxView<'_> {}

pub fn get_client<T>(
    accessor: &Accessor<T, WasiMessaging>, self_: &Resource<ClientProxy>,
) -> Result<ClientProxy> {
    accessor.with(|mut store| {
        let client = store.get().table.get(self_)?;
        Ok::<_, Error>(client.clone())
    })
}

pub fn get_message<T>(
    accessor: &Accessor<T, WasiMessaging>, self_: &Resource<MessageProxy>,
) -> Result<MessageProxy> {
    accessor.with(|mut store| {
        let message = store.get().table.get(self_)?;
        Ok::<_, Error>(message.clone())
    })
}
