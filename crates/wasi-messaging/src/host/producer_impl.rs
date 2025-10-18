use wasmtime::component::{Accessor, Resource};

use crate::host::generated::wasi::messaging::producer;
pub use crate::host::generated::wasi::messaging::types::Error;
use crate::host::generated::wasi::messaging::types::Topic;
use crate::host::resource::{ClientProxy, Message};
use crate::host::{Host, HostData, Result};

// *** WASIP3 ***
// use `HostWithStore` to add async support`

impl producer::Host for Host<'_> {}

/// The producer interface is used to send messages to a channel/topic.
impl producer::HostWithStore for HostData {
    /// Sends the message using the given client.
    async fn send<T>(
        accessor: &Accessor<T, Self>, c: Resource<ClientProxy>, topic: Topic,
        message: Resource<Message>,
    ) -> Result<()> {
        tracing::trace!("producer::Host::send: topic {topic:?}");

        let (client, msg) = accessor.with(|mut access| {
            let table = access.get().table;
            let client = table.get(&c)?;
            let msg = table.get(&message)?;
            Ok::<_, Error>((client.clone(), msg.clone()))
        })?;

        client.send(topic, msg).await?;
        Ok(())
    }
}
