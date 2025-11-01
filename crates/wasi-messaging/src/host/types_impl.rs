use wasmtime::component::{Accessor, Resource};

use crate::host::generated::wasi::messaging::types;
pub use crate::host::generated::wasi::messaging::types::{
    Error, Host, HostClient, HostClientWithStore, HostMessage, HostMessageWithStore, Topic,
};
use crate::host::resource::{ClientProxy, Message, Metadata};
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
    ) -> anyhow::Result<Resource<Message>> {
        let message = Message::new().payload(data);
        Ok(accessor.with(|mut store| store.get().table.push(message))?)
    }

    /// The topic/subject/channel this message was received on, if any.
    async fn topic<T>(
        accessor: &Accessor<T, Self>, self_: Resource<Message>,
    ) -> anyhow::Result<Option<Topic>> {
        let message = get_message(accessor, &self_)?;
        let topic = message.topic;
        if topic.is_empty() { Ok(None) } else { Ok(Some(topic)) }
    }

    /// An optional content-type describing the format of the data in the
    /// message. This is sometimes described as the "format" type".
    async fn content_type<T>(
        accessor: &Accessor<T, Self>, self_: Resource<Message>,
    ) -> anyhow::Result<Option<String>> {
        let message = get_message(accessor, &self_)?;
        let content_type = message.metadata.as_ref().and_then(|md| md.get("content-type"));
        Ok(content_type.cloned())
    }

    /// Set the content-type describing the format of the data in the message.
    /// This is sometimes described as the "format" type.
    async fn set_content_type<T>(
        accessor: &Accessor<T, Self>, self_: Resource<Message>, content_type: String,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| {
            let message = store.get().table.get_mut(&self_)?;
            message
                .metadata
                .get_or_insert_with(Metadata::new)
                .insert("content-type".to_string(), content_type);
            Ok(())
        })
    }

    /// An opaque blob of data.
    async fn data<T>(
        accessor: &Accessor<T, Self>, self_: Resource<Message>,
    ) -> anyhow::Result<Vec<u8>> {
        let message = get_message(accessor, &self_)?;
        Ok(message.payload)
    }

    /// Set the opaque blob of data for this message, discarding the old value".
    async fn set_data<T>(
        accessor: &Accessor<T, Self>, self_: Resource<Message>, data: Vec<u8>,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| {
            let message = store.get().table.get_mut(&self_)?;
            message.length = data.len();
            message.payload = data;
            Ok(())
        })
    }

    async fn metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<Message>,
    ) -> anyhow::Result<Option<types::Metadata>> {
        let message = get_message(accessor, &self_)?;
        let md = message.metadata.map(Into::into);
        Ok(md)
    }

    async fn add_metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<Message>, key: String, value: String,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| {
            let message = store.get().table.get_mut(&self_)?;
            message.metadata.get_or_insert_with(Metadata::new).insert(key, value);
            Ok(())
        })
    }

    async fn set_metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<Message>, meta: types::Metadata,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| {
            let message = store.get().table.get_mut(&self_)?;
            message.metadata = Some(meta.into());
            Ok(())
        })
    }

    async fn remove_metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<Message>, key: String,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| {
            let message = store.get().table.get_mut(&self_)?;
            if let Some(existing) = message.metadata.as_mut() {
                existing.remove(&key);
            }
            Ok(())
        })
    }

    async fn drop<T>(accessor: &Accessor<T, Self>, rep: Resource<Message>) -> anyhow::Result<()> {
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
    accessor: &Accessor<T, WasiMessaging>, self_: &Resource<Message>,
) -> Result<Message> {
    accessor.with(|mut store| {
        let message = store.get().table.get(self_)?;
        Ok::<_, Error>(message.clone())
    })
}
