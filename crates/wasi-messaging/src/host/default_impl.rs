//! Default in-memory implementation for wasi-messaging
//!
//! This is a lightweight implementation for development use only.

// #![allow(clippy::significant_drop_tightening)]
// #![allow(clippy::used_underscore_binding)]
// #![allow(clippy::assigning_clones)]
// #![allow(clippy::semicolon_outside_block)]

use std::any::Any;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::FutureExt;
use futures::stream::StreamExt;
use kernel::Backend;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio_stream::wrappers::BroadcastStream;
use tracing::instrument;

use crate::host::WasiMessagingCtx;
use crate::host::resource::{
    Client, FutureResult, Message, MessageProxy, Metadata, Reply, RequestOptions, Subscriptions,
};

#[derive(Debug, Clone, Default)]
pub struct ConnectOptions;

impl kernel::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct WasiMessagingCtxImpl {
    tx: Sender<MessageProxy>,
    rx: Receiver<MessageProxy>,
}

impl Clone for WasiMessagingCtxImpl {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            rx: self.tx.subscribe(),
        }
    }
}

impl Backend for WasiMessagingCtxImpl {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        tracing::debug!("initializing in-memory messaging");

        let (tx, rx) = broadcast::channel::<MessageProxy>(32);
        Ok(Self { tx, rx })
    }
}

impl WasiMessagingCtx for WasiMessagingCtxImpl {
    fn connect(&self) -> FutureResult<Arc<dyn Client>> {
        tracing::debug!("connecting messaging client");

        let client = self.clone();
        async move { Ok(Arc::new(client) as Arc<dyn Client>) }.boxed()
    }

    fn new_message(&self, data: Vec<u8>) -> FutureResult<Arc<dyn Message>> {
        tracing::debug!("creating new message");

        let message = InMemoryMessage::from(data);
        async move { Ok(Arc::new(message) as Arc<dyn Message>) }.boxed()
    }

    fn set_content_type(
        &self, message: Arc<dyn Message>, content_type: String,
    ) -> FutureResult<Arc<dyn Message>> {
        tracing::debug!("setting content-type: {}", content_type);

        async move {
            let Some(inmem) = message.as_any().downcast_ref::<InMemoryMessage>() else {
                anyhow::bail!("invalid message type");
            };

            let mut updated = inmem.clone();
            let mut metadata = updated.metadata.unwrap_or_default();
            metadata.insert("content-type".to_string(), content_type);
            updated.metadata = Some(metadata);

            Ok(Arc::new(updated) as Arc<dyn Message>)
        }
        .boxed()
    }

    fn set_payload(
        &self, message: Arc<dyn Message>, data: Vec<u8>,
    ) -> FutureResult<Arc<dyn Message>> {
        tracing::debug!("setting payload");

        async move {
            let Some(inmem) = message.as_any().downcast_ref::<InMemoryMessage>() else {
                anyhow::bail!("invalid message type");
            };

            let mut updated = inmem.clone();
            updated.payload = data;

            Ok(Arc::new(updated) as Arc<dyn Message>)
        }
        .boxed()
    }

    fn add_metadata(
        &self, message: Arc<dyn Message>, key: String, value: String,
    ) -> FutureResult<Arc<dyn Message>> {
        tracing::debug!("adding metadata: {key} = {value}");
        async move {
            let Some(inmem) = message.as_any().downcast_ref::<InMemoryMessage>() else {
                anyhow::bail!("invalid message type");
            };

            let mut updated = inmem.clone();
            let mut metadata = updated.metadata.unwrap_or_default();
            metadata.insert(key, value);
            updated.metadata = Some(metadata);

            Ok(Arc::new(updated) as Arc<dyn Message>)
        }
        .boxed()
    }

    fn set_metadata(
        &self, message: Arc<dyn Message>, metadata: Metadata,
    ) -> FutureResult<Arc<dyn Message>> {
        tracing::debug!("setting all metadata");
        async move {
            let inmem = message
                .as_any()
                .downcast_ref::<InMemoryMessage>()
                .ok_or_else(|| anyhow!("invalid message type"))?;

            let mut updated = inmem.clone();
            updated.metadata = Some(metadata);

            Ok(Arc::new(updated) as Arc<dyn Message>)
        }
        .boxed()
    }

    fn remove_metadata(
        &self, message: Arc<dyn Message>, key: String,
    ) -> FutureResult<Arc<dyn Message>> {
        tracing::debug!("removing metadata: {}", key);
        async move {
            let Some(inmem) = message.as_any().downcast_ref::<InMemoryMessage>() else {
                anyhow::bail!("invalid message type");
            };

            let mut updated = inmem.clone();
            if let Some(ref mut metadata) = updated.metadata {
                metadata.remove(&key);
            }

            Ok(Arc::new(updated) as Arc<dyn Message>)
        }
        .boxed()
    }
}

impl Client for WasiMessagingCtxImpl {
    fn subscribe(&self) -> FutureResult<Subscriptions> {
        tracing::debug!("subscribing to messages");

        let stream = BroadcastStream::new(self.rx.resubscribe());

        async move {
            //  TODO: replace panic with proper error handling
            let stream =
                stream.map(|res| res.unwrap_or_else(|_| panic!("failed to receive message")));

            Ok(Box::pin(stream) as Subscriptions)
        }
        .boxed()
    }

    fn send(&self, topic: String, message: MessageProxy) -> FutureResult<()> {
        tracing::debug!("sending message to topic: {topic}");
        let sender = self.tx.clone();

        async move {
            let Some(inmem) = message.as_any().downcast_ref::<InMemoryMessage>() else {
                anyhow::bail!("invalid message type");
            };

            let mut updated = inmem.clone();
            updated.topic.clone_from(&topic);
            let msg_proxy = MessageProxy(Arc::new(updated) as Arc<dyn Message>);

            sender.send(msg_proxy).map_err(|e| anyhow!("send error: {e}"))?;

            Ok(())
        }
        .boxed()
    }

    fn request(
        &self, topic: String, message: MessageProxy, _options: Option<RequestOptions>,
    ) -> FutureResult<MessageProxy> {
        tracing::debug!("sending request to topic: {}", topic);
        let sender = self.tx.clone();

        async move {
            // In a real implementation, this would send a request and wait for a response
            // For the default impl, we'll just create a simple response
            let Some(inmem) = message.as_any().downcast_ref::<InMemoryMessage>() else {
                anyhow::bail!("invalid message type");
            };

            let mut updated = inmem.clone();
            updated.topic.clone_from(&topic);

            let msg_proxy = MessageProxy(Arc::new(updated) as Arc<dyn Message>);
            sender.send(msg_proxy).map_err(|e| anyhow!("send error: {e}"))?;

            // Return a simple acknowledgment message
            let response = InMemoryMessage {
                topic: "response".to_string(),
                payload: b"ACK".to_vec(),
                metadata: None,
                description: Some("default response".to_string()),
                reply: None,
            };

            Ok(MessageProxy(Arc::new(response)))
        }
        .boxed()
    }
}

#[derive(Debug, Clone, Default)]
struct InMemoryMessage {
    topic: String,
    payload: Vec<u8>,
    metadata: Option<Metadata>,
    description: Option<String>,
    reply: Option<Reply>,
}

impl From<Vec<u8>> for InMemoryMessage {
    fn from(data: Vec<u8>) -> Self {
        Self {
            topic: String::new(),
            payload: data,
            metadata: None,
            description: None,
            reply: None,
        }
    }
}

impl Message for InMemoryMessage {
    fn topic(&self) -> String {
        self.topic.clone()
    }

    fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }

    fn metadata(&self) -> Option<Metadata> {
        self.metadata.clone()
    }

    fn description(&self) -> Option<String> {
        self.description.clone()
    }

    fn length(&self) -> usize {
        self.payload.len()
    }

    fn reply(&self) -> Option<Reply> {
        self.reply.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_messaging() {
        let ctx = WasiMessagingCtxImpl::connect_with(ConnectOptions).await.expect("connect");

        // Test connect
        let client = ctx.connect().await.expect("connect client");

        // Test new_message
        let message = ctx.new_message(b"test payload".to_vec()).await.expect("new message");
        assert_eq!(message.payload(), b"test payload".to_vec());
        assert_eq!(message.length(), 12);

        // Test set_content_type
        let message = ctx
            .set_content_type(message, "application/json".to_string())
            .await
            .expect("set content type");
        assert!(message.metadata().is_some());

        // Test add_metadata
        let message = ctx
            .add_metadata(message, "custom-key".to_string(), "custom-value".to_string())
            .await
            .expect("add metadata");
        let metadata = message.metadata().expect("metadata");
        assert_eq!(metadata.get("custom-key"), Some(&"custom-value".to_string()));

        // Test send
        client.send("test-topic".to_string(), MessageProxy(message)).await.expect("send");
    }
}
