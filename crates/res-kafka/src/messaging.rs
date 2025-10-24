//! Messaging implmentation using Kafka
use wasi_messaging::{Client, FutureResult, RequestOptions, Subscriptions};

use crate::{CLIENT_NAME, KafkaClient};

impl Client for KafkaClient {
    fn name(&self) -> &'static str {
        CLIENT_NAME
    }

    fn subscribe(&self, topics: Vec<String>) -> FutureResult<Subscriptions> {
        todo!()
    }

    fn send(&self, topic: String, message: wasi_messaging::Message) -> FutureResult<()> {
        todo!()
    }

    fn request(
        &self, topic: String, message: wasi_messaging::Message, options: Option<RequestOptions>,
    ) -> FutureResult<wasi_messaging::Message> {
        todo!()
    }
}
