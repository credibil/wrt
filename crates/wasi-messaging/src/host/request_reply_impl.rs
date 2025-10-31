use std::time::Duration;

use wasmtime::component::Resource;

use crate::host::generated::wasi::messaging::types::{HostMessage, Topic};
use crate::host::generated::wasi::messaging::{request_reply, types};
use crate::host::resource::{Message, Metadata, RequestOptions};
use crate::host::{ClientProxy, Host, Result};

/// The request-reply interface is used to send a request and receive a reply.
impl request_reply::Host for Host<'_> {
    /// Performs a request-reply operation using the given client and options
    /// (if any).
    async fn request(
        &mut self, c: Resource<ClientProxy>, topic: Topic, message: Resource<Message>,
        options: Option<Resource<RequestOptions>>,
    ) -> Result<Vec<Resource<Message>>> {
        tracing::trace!("request_reply::Host::request: topic {:?}", topic);

        let client = self.table.get(&c)?;
        let request = self.table.get(&message)?.clone();

        let options = if let Some(opts) = options {
            let options = self.table.get(&opts)?;
            Some(options.clone())
        } else {
            None
        };

        let reply = client.request(topic, request, options).await?;
        Ok(vec![self.table.push(reply)?])
    }

    /// Replies to the given message with the given response message.
    async fn reply(
        &mut self, reply_to: Resource<Message>, message: Resource<Message>,
    ) -> Result<()> {
        tracing::trace!("request_reply::Host::reply");

        let reply_to = self.table.get(&reply_to)?;

        if let Some(reply) = &reply_to.reply {
            let message = self.table.get(&message)?.clone();
            let client = ClientProxy::try_from(&reply.client_name)?;
            let topic = reply.topic.clone();
            client.send(topic.clone(), message).await?;
        }

        Ok(())
    }
}

impl request_reply::HostRequestOptions for Host<'_> {
    /// Creates a new request options resource with no options set.
    async fn new(&mut self) -> anyhow::Result<Resource<RequestOptions>> {
        tracing::trace!("request_reply::HostRequestOptions::new");
        let options = RequestOptions::default();
        Ok(self.table.push(options)?)
    }

    /// The maximum amount of time to wait for a response. If the timeout value
    /// is not set, then the request/reply operation will block until a message
    /// is received in response.
    async fn set_timeout_ms(
        &mut self, self_: Resource<RequestOptions>, timeout_ms: u32,
    ) -> anyhow::Result<()> {
        tracing::trace!("request_reply::HostRequestOptions::set_timeout_ms {timeout_ms}");
        let options = self.table.get_mut(&self_)?;
        options.timeout = Some(Duration::from_millis(u64::from(timeout_ms)));
        Ok(())
    }

    /// The maximum number of replies to expect before returning.
    ///
    /// For NATS, this is not configurable so this function does nothing.
    async fn set_expected_replies(
        &mut self, self_: Resource<RequestOptions>, expected_replies: u32,
    ) -> anyhow::Result<()> {
        let options = self.table.get_mut(&self_)?;
        options.expected_replies = Some(expected_replies);
        Ok(())
    }

    /// Removes the resource from the resource table.
    async fn drop(&mut self, rep: Resource<RequestOptions>) -> anyhow::Result<()> {
        tracing::trace!("request_reply::HostRequestOptions::drop");
        self.table.delete(rep).map(|_| Ok(()))?
    }
}

impl HostMessage for Host<'_> {
    /// Create a new message with the given payload.
    async fn new(&mut self, data: Vec<u8>) -> anyhow::Result<Resource<Message>> {
        tracing::trace!("HostMessage::new with {} bytes", data.len());
        let msg = Message::new().payload(data);
        Ok(self.table.push(msg)?)
    }

    /// The topic/subject/channel this message was received on, if any.
    async fn topic(&mut self, self_: Resource<Message>) -> anyhow::Result<Option<Topic>> {
        tracing::trace!("HostMessage::topic");
        let msg = self.table.get(&self_)?;
        let topic = msg.topic.clone();
        if topic.is_empty() { Ok(None) } else { Ok(Some(topic)) }
    }

    /// An optional content-type describing the format of the data in the
    /// message. This is sometimes described as the "format" type".
    async fn content_type(&mut self, self_: Resource<Message>) -> anyhow::Result<Option<String>> {
        tracing::trace!("HostMessage::content_type");
        let msg = self.table.get(&self_)?;
        let content_type = msg.metadata.as_ref().and_then(|h| h.get("content-type"));
        content_type.map_or_else(
            || {
                let content_type = msg.metadata.as_ref().and_then(|h| h.get("Content-Type"));
                content_type.map_or_else(|| Ok(None), |ct| Ok(Some(ct.to_string())))
            },
            |ct| Ok(Some(ct.to_string())),
        )
    }

    /// Set the content-type describing the format of the data in the message.
    /// This is sometimes described as the "format" type.
    async fn set_content_type(
        &mut self, self_: Resource<Message>, content_type: String,
    ) -> anyhow::Result<()> {
        tracing::trace!("HostMessage::set_content_type {content_type}");
        let msg = self.table.get_mut(&self_)?;
        let mut metadata = msg.metadata.take().unwrap_or_default();
        metadata.insert("content-type".to_string(), content_type);
        msg.metadata = Some(metadata);
        Ok(())
    }

    /// An opaque blob of data.
    async fn data(&mut self, self_: Resource<Message>) -> anyhow::Result<Vec<u8>> {
        tracing::trace!("HostMessage::data");
        let msg = self.table.get(&self_)?;
        Ok(msg.payload.clone())
    }

    /// Set the opaque blob of data for this message, discarding the old value".
    async fn set_data(&mut self, self_: Resource<Message>, data: Vec<u8>) -> anyhow::Result<()> {
        tracing::trace!("HostMessage::set_data");
        let msg = self.table.get_mut(&self_)?;
        msg.length = data.len();
        msg.payload = data;
        Ok(())
    }

    async fn metadata(
        &mut self, self_: Resource<types::Message>,
    ) -> anyhow::Result<Option<types::Metadata>> {
        tracing::trace!("HostMessage::metadata");
        let message = self.table.get(&self_)?;
        message.metadata.as_ref().map_or_else(|| Ok(None), |m| Ok(Some(m.into())))
    }

    async fn add_metadata(
        &mut self, self_: Resource<Message>, key: String, value: String,
    ) -> anyhow::Result<()> {
        tracing::trace!("HostMessage::add_metadata {key}={value}");
        let message = self.table.get_mut(&self_)?;
        message.metadata.get_or_insert_with(Metadata::new).insert(key, value);
        Ok(())
    }

    async fn set_metadata(
        &mut self, self_: Resource<Message>, meta: types::Metadata,
    ) -> anyhow::Result<()> {
        tracing::trace!("HostMessage::set_metadata");
        let message = self.table.get_mut(&self_)?;
        message.metadata = Some(meta.into());
        Ok(())
    }

    async fn remove_metadata(
        &mut self, self_: Resource<Message>, key: String,
    ) -> anyhow::Result<()> {
        tracing::trace!("HostMessage::remove_metadata {key}");
        let message = self.table.get_mut(&self_)?;

        let existing = message.metadata.as_mut();
        if let Some(existing) = existing {
            existing.remove(&key);
        }

        Ok(())
    }

    async fn drop(&mut self, rep: Resource<Message>) -> anyhow::Result<()> {
        tracing::trace!("HostMessage::drop");
        self.table.delete(rep)?;
        Ok(())
    }
}
