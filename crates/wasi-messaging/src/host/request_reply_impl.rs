use std::time::Duration;

use wasmtime::component::{Accessor, Resource};

use crate::host::generated::wasi::messaging::request_reply::{
    Host, HostRequestOptions, HostRequestOptionsWithStore, HostWithStore,
};
use crate::host::generated::wasi::messaging::types::Topic;
use crate::host::resource::{ClientProxy, Message, RequestOptions};
use crate::host::types_impl::{get_client, get_message};
use crate::host::{Result, WasiMessaging, WasiMessagingCtxView};

impl HostWithStore for WasiMessaging {
    async fn request<T>(
        accessor: &Accessor<T, Self>, c: Resource<ClientProxy>, topic: Topic,
        message: Resource<Message>, options: Option<Resource<RequestOptions>>,
    ) -> Result<Vec<Resource<Message>>> {
        let client = get_client(accessor, &c)?;
        let request = get_message(accessor, &message)?;
        let options = accessor.with(|mut access| {
            let options = if let Some(opts) = options {
                let options = access.get().table.get(&opts)?;
                Some(options.clone())
            } else {
                None
            };
            Ok::<_, anyhow::Error>(options)
        })?;

        let reply = client.request(topic, request, options).await?;
        let reply_res = accessor.with(|mut access| access.get().table.push(reply))?;

        Ok(vec![reply_res])
    }

    /// Replies to the given message with the given response message.
    async fn reply<T>(
        accessor: &Accessor<T, Self>, reply_to: Resource<Message>, message: Resource<Message>,
    ) -> Result<()> {
        let reply_to = get_message(accessor, &reply_to)?;
        let Some(reply) = &reply_to.reply else { return Ok(()) };

        let client = accessor.with(|mut store| store.get().ctx.connect()).await?;
        let message = get_message(accessor, &message)?;

        client.send(reply.topic.clone(), message).await?;

        Ok(())
    }
}

impl HostRequestOptionsWithStore for WasiMessaging {
    /// Creates a new request options resource with no options set.
    async fn new<T>(accessor: &Accessor<T, Self>) -> anyhow::Result<Resource<RequestOptions>> {
        let options = RequestOptions::default();
        Ok(accessor.with(|mut store| store.get().table.push(options))?)
    }

    /// The maximum amount of time to wait for a response. If the timeout value
    /// is not set, then the request/reply operation will block until a message
    /// is received in response.
    async fn set_timeout_ms<T>(
        accessor: &Accessor<T, Self>, self_: Resource<RequestOptions>, timeout_ms: u32,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| {
            let options = store.get().table.get_mut(&self_)?;
            options.timeout = Some(Duration::from_millis(u64::from(timeout_ms)));
            Ok(())
        })
    }

    /// The maximum number of replies to expect before returning.
    ///
    /// For NATS, this is not configurable so this function does nothing.
    async fn set_expected_replies<T>(
        accessor: &Accessor<T, Self>, self_: Resource<RequestOptions>, expected_replies: u32,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| {
            let options = store.get().table.get_mut(&self_)?;
            options.expected_replies = Some(expected_replies);
            Ok(())
        })
    }

    /// Removes the resource from the resource table.
    async fn drop<T>(
        accessor: &Accessor<T, Self>, rep: Resource<RequestOptions>,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

impl Host for WasiMessagingCtxView<'_> {}
impl HostRequestOptions for WasiMessagingCtxView<'_> {}
