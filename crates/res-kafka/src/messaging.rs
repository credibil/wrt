use std::collections::HashMap;
use std::pin::Pin;

use futures::future::FutureExt;
use futures::stream::StreamExt;
use futures::task::{Context, Poll};
use futures::{Stream, TryStreamExt};
use rdkafka::Message as _;
use rdkafka::consumer::Consumer;
use rdkafka::message::{BorrowedMessage, Headers};
use rdkafka::producer::BaseRecord;
use tokio::sync::mpsc;
use wasi_messaging::{Client, FutureResult, Message, Metadata, RequestOptions, Subscriptions};

use crate::Client as Kafka;

const CAPACITY: usize = 1024;

impl Client for Kafka {
    fn subscribe(&self, topics: Vec<String>) -> FutureResult<Subscriptions> {
        let client = self.clone();

        async move {
            let consumer = client.consumer;

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
                        async move {
                            let message = into_message(&msg);
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

            // custom partitioning when available AND message doesn't specify partition
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

            // TODO: this looks redundant??
            // let p: i32 = msg.partition();
            // if p >= 0 {
            //     record = record.partition(p);
            // }

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

fn into_message(kafka_msg: &BorrowedMessage<'_>) -> Message {
    let metadata = kafka_msg.headers().map(|headers| {
        let mut md = HashMap::new();
        for h in headers.iter() {
            let bytes = h.value.unwrap_or_default();
            let v = String::from_utf8_lossy(bytes).to_string();
            md.insert(h.key.to_string(), v);
        }
        Metadata { inner: md }
    });

    let topic = kafka_msg.topic().to_string();
    let payload = kafka_msg.payload().unwrap_or_default().to_vec();
    let length = payload.len();

    Message {
        topic,
        payload,
        metadata,
        length,
        ..Message::default()
    }
}
