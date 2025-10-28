#![cfg(target_arch = "wasm32")]

use tracing::Level;
use wasi_messaging_kafka::incoming_handler::Configuration;
use wasi_messaging_kafka::types::{Error, Message};

pub struct Messaging;

impl wasi_messaging_kafka::incoming_handler::Guest for Messaging {
    /// To be re-enabled when the otel issue is resolved
    #[wasi_otel::instrument(name = "messaging_guest_handle",level = Level::DEBUG)]
    async fn handle(message: Message) -> anyhow::Result<(), Error> {
        println!("start processing msg");
        let topic = message.topic().unwrap_or_default();
        let data = message.data();
        let msg = String::from_utf8(data).unwrap_or_default();
        println!("message processed for topic: {}, body: {}", topic, msg);
        Ok(())
    }

    async fn configure() -> Result<Configuration, Error> {
        Ok(Configuration {
            topics: vec!["tst-realtime-gtfs-tu.v1".to_string()],
        })
    }
}

wasi_messaging_kafka::export!(Messaging with_types_in wasi_messaging_kafka);
