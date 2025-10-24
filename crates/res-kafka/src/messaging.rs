//! Messaging implmentation using Kafka

use std::collections::HashMap;

use anyhow::anyhow;
use futures::{FutureExt, StreamExt};
use rdkafka::error::KafkaResult;
use rdkafka::Message as _;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::{BorrowedMessage, Headers};
use wasi_messaging::{Client, FutureResult, Message, Metadata, RequestOptions, Subscriptions};

use crate::{CLIENT_NAME, KafkaClient};

impl Client for KafkaClient {
    fn name(&self) -> &'static str {
        CLIENT_NAME
    }

    fn subscribe(&self, topics: Vec<String>) -> FutureResult<Subscriptions> {
        let config = self.config.clone();
        async move {
            let consumer: StreamConsumer =
                config.create().map_err(|e| anyhow!("failed to create consumer: {e}"))?;
            consumer.subscribe(&topics.iter().map(|s| &**s).collect::<Vec<&str>>())?;
            let stream = consumer.stream().map(into_message);
            Ok(Box::pin(stream) as Subscriptions)
        }
        .boxed()
    }

    fn pre_send(&self, message: &Message) -> FutureResult<()> {
        todo!()
    }

    fn send(&self, topic: String, message: Message) -> FutureResult<()> {
        todo!()
    }

    fn post_send(&self, message: &Message) -> FutureResult<()> {
        todo!()
    }

    fn request(
        &self, topic: String, message: Message, options: Option<RequestOptions>,
    ) -> FutureResult<wasi_messaging::Message> {
        todo!()
    }
}

fn into_message(kafka_message: KafkaResult<BorrowedMessage>) -> Message {
    let Ok(kafka_message) = kafka_message else {
        return Message::default();
    };

    let metadata = kafka_message.headers().map(|headers| {
        let mut header_map = HashMap::new();
        for h in headers.iter() {
            let key = h.key.to_string();
            let value = h.value.map(|v| String::from_utf8_lossy(v).to_string()).unwrap_or_default();
            header_map.insert(key, value);
        }
        Metadata { inner: header_map }
    });

    Message {
        topic: kafka_message.topic().to_string(),
        payload: kafka_message.payload().map(|p| p.to_vec()).unwrap_or_default(),
        metadata,
        description: None,
        length: kafka_message.payload_len(),
        reply: None,
        ..Default::default()
    }
}
