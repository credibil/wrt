use std::collections::HashMap;

use anyhow::anyhow;
use futures::future::FutureExt;
use futures::stream::{self, StreamExt};
use wasi_messaging::{
    Client, FutureResult, Message, Metadata, Reply, RequestOptions, Subscriptions,
};

use crate::{CLIENT_NAME, Client as Nats};

impl Client for Nats {
    fn name(&self) -> &'static str {
        CLIENT_NAME
    }

    fn subscribe(&self, topics: Vec<String>) -> FutureResult<Subscriptions> {
        let client = self.0.clone();

        async move {
            tracing::trace!("subscribing to messaging topics: {topics:?}");

            // topics to subscribe to
            let mut subscribers = vec![];

            for t in &topics {
                tracing::debug!("subscribing to {t}");
                let subscriber = client.subscribe(t.clone()).await?;
                subscribers.push(subscriber);
            }

            tracing::info!("subscribed to {topics:?}");

            // process messages until terminated
            let stream = stream::select_all(subscribers).map(into_message);
            Ok(Box::pin(stream) as Subscriptions)
        }
        .boxed()
    }

    fn send(&self, topic: String, message: Message) -> FutureResult<()> {
        let client = self.0.clone();

        async move {
            let Some(headers) = message.metadata.clone() else {
                client
                    .publish(topic.clone(), message.payload.into())
                    .await
                    .map_err(|e| anyhow!("failed to publish: {e}"))?;
                return Ok(());
            };

            let mut nats_headers = async_nats::HeaderMap::new();
            for (k, v) in headers.iter() {
                nats_headers.insert(k.as_str(), v.as_str());
            }

            client
                .publish_with_headers(topic.clone(), nats_headers, message.payload.into())
                .await
                .map_err(|e| anyhow!("failed to publish: {e}"))?;

            Ok(())
        }
        .boxed()
    }

    fn request(
        &self, topic: String, message: Message, options: Option<RequestOptions>,
    ) -> FutureResult<Message> {
        let client = self.0.clone();

        async move {
            let payload = message.payload.clone();
            let headers = message.metadata.clone().unwrap_or_default();
            let mut nats_headers = async_nats::HeaderMap::new();
            for (k, v) in headers.iter() {
                nats_headers.insert(k.as_str(), v.as_str());
            }

            let timeout = if let Some(request_options) = options
                && request_options.timeout.is_some()
            {
                request_options.timeout
            } else {
                None
            };

            let request = async_nats::Request::new()
                .payload(payload.into())
                .headers(nats_headers)
                .timeout(timeout);

            let nats_msg = client
                .send_request(topic.clone(), request)
                .await
                .map_err(|e| anyhow!("failed to send request: {e}"))?;

            Ok(into_message(nats_msg))
        }
        .boxed()
    }
}

fn into_message(nats_msg: async_nats::Message) -> Message {
    let metadata = nats_msg.headers.map(|headers| {
        let mut header_map = HashMap::new();
        for (k, v) in headers.iter() {
            let v = v.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
            header_map.insert(k.to_string(), v);
        }
        Metadata { inner: header_map }
    });

    let reply = nats_msg.reply.map(|reply| Reply {
        client_name: CLIENT_NAME.to_string(),
        topic: reply.to_string(),
    });

    Message {
        topic: nats_msg.subject.to_string(),
        payload: nats_msg.payload.to_vec(),
        metadata,
        description: None,
        length: nats_msg.payload.len(),
        reply,
    }
}
