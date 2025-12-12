use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use async_nats::HeaderMap;
use futures::future::FutureExt;
use futures::stream::{self, StreamExt};
use wasi_messaging::{
    Client, FutureResult, Message, MessageProxy, Metadata, Reply, RequestOptions, Subscriptions,
    WasiMessagingCtx,
};

impl WasiMessagingCtx for crate::Client {
    fn connect(&self) -> FutureResult<Arc<dyn Client>> {
        let client = self.clone();
        async move { Ok(Arc::new(client) as Arc<dyn Client>) }.boxed()
    }

    fn new_message(&self, data: Vec<u8>) -> anyhow::Result<Arc<dyn Message>> {
        let length = data.len();

        let msg = async_nats::Message {
            subject: String::new().into(),
            reply: None,
            payload: data.into(),
            headers: None,
            status: None,
            description: None,
            length,
        };
        Ok(Arc::new(NatsMessage(msg)) as Arc<dyn Message>)
    }

    fn set_content_type(
        &self, message: Arc<dyn Message>, content_type: String,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let nats_msg = message
            .as_any()
            .downcast_ref::<NatsMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let mut msg = nats_msg.0.clone();
        let mut headers = nats_msg.0.headers.clone().unwrap_or_default();
        headers.insert("content-type".to_string(), content_type);
        msg.headers = Some(headers);
        Ok(Arc::new(NatsMessage(msg)) as Arc<dyn Message>)
    }

    fn set_payload(
        &self, message: Arc<dyn Message>, data: Vec<u8>,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let nats_msg = message
            .as_any()
            .downcast_ref::<NatsMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let mut msg = nats_msg.0.clone();
        msg.payload = data.clone().into();
        msg.length = data.len();
        Ok(Arc::new(NatsMessage(msg)) as Arc<dyn Message>)
    }

    fn add_metadata(
        &self, message: Arc<dyn Message>, key: String, value: String,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let nats_msg = message
            .as_any()
            .downcast_ref::<NatsMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let mut msg = nats_msg.0.clone();
        let mut headers = nats_msg.0.headers.clone().unwrap_or_default();
        headers.insert(key, value);
        msg.headers = Some(headers);
        Ok(Arc::new(NatsMessage(msg)) as Arc<dyn Message>)
    }

    fn set_metadata(
        &self, message: Arc<dyn Message>, metadata: Metadata,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let nats_msg = message
            .as_any()
            .downcast_ref::<NatsMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let mut msg = nats_msg.0.clone();
        let mut headers = async_nats::HeaderMap::new();
        for (k, v) in &metadata.inner {
            headers.insert(k.as_str(), v.as_str());
        }
        msg.headers = Some(headers);
        Ok(Arc::new(NatsMessage(msg)) as Arc<dyn Message>)
    }

    fn remove_metadata(
        &self, message: Arc<dyn Message>, key: String,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let nats_msg = message
            .as_any()
            .downcast_ref::<NatsMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let mut msg = nats_msg.0.clone();
        if let Some(headers) = nats_msg.0.headers.clone() {
            let mut updated_headers = HeaderMap::new();
            for (k, v) in headers.iter() {
                if k.to_string() != key {
                    for iv in v {
                        updated_headers.insert(k.clone(), iv.clone());
                    }
                }
            }
            msg.headers = Some(updated_headers);
        }
        Ok(Arc::new(NatsMessage(msg)) as Arc<dyn Message>)
    }
}

#[derive(Clone, Debug)]
struct NatsMessage(async_nats::Message);

impl Message for NatsMessage {
    fn topic(&self) -> String {
        self.0.subject.to_string()
    }

    fn payload(&self) -> Vec<u8> {
        self.0.payload.to_vec()
    }

    fn metadata(&self) -> Option<Metadata> {
        let mut md = HashMap::new();
        for (k, v) in self.0.headers.as_ref()?.iter() {
            let v_str = v.iter().map(ToString::to_string).collect::<Vec<String>>().join(", ");
            md.insert(k.to_string(), v_str);
        }
        Some(Metadata { inner: md })
    }

    fn description(&self) -> Option<String> {
        self.0.description.clone()
    }

    fn length(&self) -> usize {
        self.0.length
    }

    fn reply(&self) -> Option<Reply> {
        self.0.reply.clone().map(|r| Reply {
            client_name: String::new(),
            topic: r.to_string(),
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Client for crate::Client {
    fn subscribe(&self) -> FutureResult<Subscriptions> {
        let client = self.clone();

        async move {
            let Some(topics) = client.topics else {
                return Err(anyhow!("No topics specified"));
            };

            let mut subscribers = vec![];
            for t in &topics {
                let subscriber = client.inner.subscribe(t.clone()).await?;
                subscribers.push(subscriber);
            }

            tracing::info!("subscribed to {topics:?} topics");

            // process messages until terminated
            let stream = stream::select_all(subscribers)
                .map(|msg| MessageProxy(Arc::new(NatsMessage(msg)) as Arc<dyn Message>));
            Ok(Box::pin(stream) as Subscriptions)
        }
        .boxed()
    }

    fn send(&self, topic: String, message: MessageProxy) -> FutureResult<()> {
        let client = self.inner.clone();
        async move {
            let payload = message.payload();
            let Some(headers) = message.metadata() else {
                client.publish(topic.clone(), payload.into()).await.context("failed to publish")?;
                return Ok(());
            };

            let mut nats_headers = async_nats::HeaderMap::new();
            for (k, v) in headers.iter() {
                nats_headers.insert(k.as_str(), v.as_str());
            }

            client
                .publish_with_headers(topic.clone(), nats_headers, payload.into())
                .await
                .context("failed to publish")?;

            Ok(())
        }
        .boxed()
    }

    fn request(
        &self, topic: String, message: MessageProxy, options: Option<RequestOptions>,
    ) -> FutureResult<MessageProxy> {
        let client = self.inner.clone();

        async move {
            let payload = message.payload();
            let headers = message.metadata();
            let mut nats_headers = async_nats::HeaderMap::new();
            if let Some(meta) = headers {
                for (k, v) in meta.iter() {
                    nats_headers.insert(k.as_str(), v.as_str());
                }
            }
            let timeout = options.and_then(|options| options.timeout);

            let request = async_nats::Request::new()
                .payload(payload.into())
                .headers(nats_headers)
                .timeout(timeout);

            let nats_msg = client
                .send_request(topic.clone(), request)
                .await
                .context("failed to send request")?;
            Ok(MessageProxy(Arc::new(NatsMessage(nats_msg)) as Arc<dyn Message>))
        }
        .boxed()
    }
}
