use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use futures::future::{BoxFuture, FutureExt};
use futures::stream::{self, StreamExt};
use runtime::RunState;
use tracing::{Instrument, info_span};
use wasi_messaging::{Client, Error, Message, Messaging, Metadata, Reply};
use wasmtime::Store;
use wasmtime::component::InstancePre;

const CLIENT_NAME: &str = "nats";

#[derive(Debug)]
pub struct NatsClient(pub async_nats::Client);

impl Client for NatsClient {
    fn name(&self) -> &'static str {
        CLIENT_NAME
    }

    fn subscribe(
        &self, topics: Vec<String>, instance_pre: InstancePre<RunState>,
    ) -> BoxFuture<'static, Result<()>> {
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
            let mut messages = stream::select_all(subscribers);
            while let Some(nats_msg) = messages.next().await {
                let message = to_message(nats_msg);
                let instance_pre = instance_pre.clone();

                tokio::spawn(
                    async move {
                        if let Err(e) = call_guest(message, instance_pre).await {
                            tracing::error!("error processing message {e}");
                        }
                    }
                    .instrument(info_span!("message")),
                );
            }

            Ok(())
        }
        .boxed()
    }

    fn send(&self, topic: String, message: Message) -> BoxFuture<'static, Result<()>> {
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

    fn request(&self, topic: String, message: Message) -> BoxFuture<'static, Result<Message>> {
        let client = self.0.clone();

        async move {
            let payload = message.payload.clone();
            let headers = message.metadata.clone().unwrap_or_default();
            let mut nats_headers = async_nats::HeaderMap::new();
            for (k, v) in headers.iter() {
                nats_headers.insert(k.as_str(), v.as_str());
            }
            let timeout = message.timeout.unwrap_or(None);

            let request = async_nats::Request::new()
                .payload(payload.into())
                .headers(nats_headers)
                .timeout(timeout);

            let nats_msg = client
                .send_request(topic.clone(), request)
                .await
                .map_err(|e| anyhow!("failed to send request: {e}"))?;

            Ok(to_message(nats_msg))
        }
        .boxed()
    }
}

// Forward message to the wasm component.
async fn call_guest(message: Message, instance_pre: InstancePre<RunState>) -> Result<(), Error> {
    let mut state = RunState::new();
    let res_msg = state.table.push(message)?;

    let mut store = Store::new(instance_pre.engine(), state);
    let instance = instance_pre.instantiate_async(&mut store).await?;
    let messaging = Messaging::new(&mut store, &instance)?;

    // *** WASIP3 ***
    // use `run_concurrent` for non-blocking execution
    instance
        .run_concurrent(&mut store, async |accessor| {
            messaging.wasi_messaging_incoming_handler().call_handle(accessor, res_msg).await?
        })
        .await
        .context("error running instance: {e}")?
}

fn to_message(nats_msg: async_nats::Message) -> Message {
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
        timeout: None,
    }
}
