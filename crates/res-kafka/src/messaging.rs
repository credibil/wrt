use std::collections::HashMap;
use std::env;
use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use futures::future::FutureExt;
use futures::stream::StreamExt;
use futures::task::{Context, Poll};
use rdkafka::Message as _;
use rdkafka::consumer::Consumer;
use rdkafka::message::{BorrowedMessage, Headers};
use rdkafka::producer::BaseRecord;
use tokio::sync::mpsc;
use wasi_messaging::{
    Client, FutureResult, Message, Metadata, RequestOptions, Subscriptions, WasiMessagingCtx,
};

use crate::registry::Registry;

const CAPACITY: usize = 1024;

impl WasiMessagingCtx for crate::Client {
    fn connect(&self) -> FutureResult<Arc<dyn Client>> {
        let client = self.clone();
        async move { Ok(Arc::new(client) as Arc<dyn Client>) }.boxed()
    }
}

impl Client for crate::Client {
    fn subscribe(&self) -> FutureResult<Subscriptions> {
        let client = self.clone();
        let topics_env = env::var("KAFKA_TOPICS").unwrap_or_default();
        let topics = topics_env.split(',').map(ToString::to_string).collect::<Vec<_>>();

        async move {
            let consumer = client.consumer;
            let registry = client.registry;

            // subscribe
            let topics = topics.iter().map(String::as_str).collect::<Vec<_>>();
            consumer.subscribe(&topics)?;

            // spawn a task to read messages and forward subscriber
            let (sender, receiver) = mpsc::channel::<Message>(CAPACITY);
            tokio::spawn(async move {
                consumer
                    .stream()
                    .filter_map(|res| async {
                        res.map_or_else(
                            |e| {
                                tracing::error!("kafka consumer error: {e}");
                                None
                            },
                            Some,
                        )
                    })
                    .for_each(|msg| {
                        let sender = sender.clone();
                        let registry = registry.clone();

                        async move {
                            let message = into_message(&msg, registry.as_ref()).await;
                            if let Err(e) = sender.send(message).await {
                                tracing::error!("failed to send message to subscriber: {e}");
                            }
                        }
                    })
                    .await;
            });

            Ok(Box::pin(Subscriber { receiver }) as Subscriptions)
        }
        .boxed()
    }

    fn send(&self, topic: String, message: Message) -> FutureResult<()> {
        let client = self.clone();

        // TODO: add offset to header??

        async move {
            // schema registry validation when available
            let payload = if let Some(sr) = &client.registry {
                sr.validate_and_encode_json(&topic, message.payload).await
            } else {
                message.payload
            };

            let metadata = message.metadata.unwrap_or_default();
            let now = chrono::Utc::now().timestamp_millis();

            let key = metadata.get("key").cloned().unwrap_or_default();
            let mut record =
                BaseRecord::to(&topic).payload(&payload).key(key.as_bytes()).timestamp(now);

            // partitioning
            let partition = metadata.get("partition").cloned().unwrap_or_default();
            let partition = partition.parse().unwrap_or(-1);
            if partition >= 0 {
                record = record.partition(partition);
            } else if let Some(partitioner) = &client.partitioner
                && let Some(key) = metadata.get("key")
            {
                let partition = partitioner.partition(key.as_bytes());
                record = record.partition(partition);
            }

            if let Err((e, _)) = client.producer.send(record) {
                tracing::error!("producer::error {e}");
            }

            Ok(())
        }
        .boxed()
    }

    fn request(
        &self, _topic: String, _message: Message, _options: Option<RequestOptions>,
    ) -> FutureResult<Message> {
        async move { unimplemented!() }.boxed()
    }
}

#[derive(Debug)]
pub struct Subscriber {
    receiver: mpsc::Receiver<Message>,
}

impl Stream for Subscriber {
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

async fn into_message(kafka_msg: &BorrowedMessage<'_>, registry: Option<&Registry>) -> Message {
    let metadata = kafka_msg.headers().map(|headers| {
        let mut md = HashMap::new();
        for h in headers.iter() {
            let bytes = h.value.unwrap_or_default();
            let v = String::from_utf8_lossy(bytes).to_string();
            md.insert(h.key.to_string(), v);
        }
        Metadata { inner: md }
    });

    let topic = kafka_msg.topic();
    let payload_bytes = kafka_msg.payload().unwrap_or_default().to_vec();
    
    let payload = if let Some(sr) = &registry {
        sr.validate_and_decode_json(topic, &payload_bytes).await
    } else {
        payload_bytes
    };

    let length = payload.len();

    Message {
        topic: topic.to_string(),
        payload,
        metadata,
        length,
        ..Message::default()
    }
}
