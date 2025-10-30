#![cfg(target_arch = "wasm32")]

use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::Result;
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::{Value, json};
use tracing::Level;
use wasi::exports::http;
use wasi::http::types::{IncomingRequest, ResponseOutparam};
use wasi_messaging_kafka::incoming_handler::Configuration;
use wasi_messaging_kafka::types::{Client, Error, Message};
use wasi_messaging_kafka::producer;

pub struct Http;

impl http::incoming_handler::Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let router = Router::new().route("/", post(handler));
        let out = wasi_http::serve(router, request);
        ResponseOutparam::set(response_out, out);
    }
}

#[wasi_otel::instrument]
#[axum::debug_handler]
async fn handler(body: Bytes) -> Json<Value> {
    let client = Client::connect("kafka").unwrap();
    let message = Message::new(&body);
    message.set_content_type("application/json");
    message.add_metadata("key", "example_key");
    println!("handler: sending message to topic 'a.v1'");
    wit_bindgen::spawn(async move {
        if let Err(e) = producer::send(&client, "a.v1".to_string(), message).await {
            tracing::error!("error sending message to topic 'a.v1': {e}");
        }
        println!("handler: message published to topic 'a.v1'");
    });

    Json(json!({"message": "message published"}))
}

wasi::http::proxy::export!(Http);

pub struct Messaging;

impl wasi_messaging_kafka::incoming_handler::Guest for Messaging {
    #[wasi_otel::instrument(name = "messaging_guest_handle",level = Level::DEBUG)]
    async fn handle(message: Message) -> anyhow::Result<(), Error> {
        tracing::debug!("start processing msg");
        println!("start processing msg");
        let topic = message.topic().unwrap_or_default();
        let data = message.data();
        let msg = String::from_utf8(data).unwrap_or_default();
        tracing::debug!("message processed for topic: {}, body: {}", topic, msg);
        println!("message processed for topic: {topic}, body: {msg}");
        match topic.as_str() {
            "a.v1" => {
                tracing::debug!("handling topic a.v1");
                println!("handling topic a.v1");

                // send message to topic `b.v1`
                let mut resp = b"topic a.v1 says: ".to_vec();
                resp.extend(message.data());

                let pubmsg = Message::new(&resp);
                if let Some(md) = message.metadata() {
                    pubmsg.set_metadata(&md);
                }
                if let Some(format) = message.content_type() {
                    pubmsg.set_content_type(&format);
                }

                let timer = Instant::now();

                // *** WASIP3 ***
                // use `spawn` to avoid blocking for non-blocking execution
                for i in 0..100 {
                    wit_bindgen::spawn(async move {
                        println!("sending message iteration {i}");
                        let Ok(client) = Client::connect("kafka") else {
                            tracing::error!("failed to connect kafka client");
                            println!("failed to connect kafka client");
                            return;
                        };
                        println!("client connected");
                        let data = format!("topic a iteration {i}");
                        let message = Message::new(data.as_bytes());
                        message.add_metadata("key", &format!("key{i}"));

                        if let Err(e) = producer::send(&client, "b.v1".to_string(), message).await {
                            tracing::error!("error sending message to topic 'b.v1': {e}");
                            println!("error sending message to topic 'b.v1': {e}");
                        }
                        println!("message iteration {i} sent");

                        // HACK: yield to host
                        if i % 100 == 0 {
                            sleep(Duration::from_nanos(1));
                            // wit_bindgen::yield_async().await;
                        }
                    });
                }
                println!("sent 100 messages in {} milliseconds", timer.elapsed().as_millis());
            }
            "b.v1" => {
                tracing::debug!("handling topic b.v1");
                println!("handling topic b.v1");
                // process message for topic b.v1
            }
            _ => {
                tracing::debug!("unknown topic: {}", topic);
            }
        }
        tracing::debug!("finished processing msg");
        println!("finished processing msg");
        Ok(())
    }

    async fn configure() -> Result<Configuration, Error> {
        tracing::debug!("configuring messaging guest");
        println!("configuring messaging guest");
        Ok(Configuration {
            topics: vec!["a.v1".to_string(), "b.v1".to_string()],
        })
    }
}

wasi_messaging_kafka::export!(Messaging with_types_in wasi_messaging_kafka);
