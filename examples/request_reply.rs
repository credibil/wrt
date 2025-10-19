#![cfg(target_arch = "wasm32")]

use anyhow::Result;
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::{Value, json};
use tracing::Level;
use wasi::exports::http;
use wasi::http::types::{IncomingRequest, ResponseOutparam};
use wasi_messaging::incoming_handler::Configuration;
use wasi_messaging::request_reply;
use wasi_messaging::types::{Client, Error, Message};

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
async fn handler(body: Bytes) -> Json<Value> {
    let client = Client::connect("nats").unwrap();
    let message = Message::new(&body);
    let reply = request_reply::request(&client, "a", &message, None).expect("should reply");

    // process first reply
    let data = reply[0].data();
    let data_str = String::from_utf8_lossy(&data);

    Json(json!({"reply": data_str}))
}

wasi::http::proxy::export!(Http);

pub struct RequestReply;

impl wasi_messaging::incoming_handler::Guest for RequestReply {
    #[wasi_otel::instrument(name = "messaging_guest_handle",level = Level::DEBUG)]
    async fn handle(message: Message) -> Result<(), Error> {
        match message.topic().as_deref() {
            Some("a") => {
                let data = message.data();
                let data_str = String::from_utf8(data.clone())
                    .map_err(|e| Error::Other(format!("not utf8: {e}")))?;
                tracing::debug!("message received on topic 'a': {data_str}");

                // send message to topic `b`
                let mut resp = b"reply from topic a: ".to_vec();
                resp.extend(data);

                let reply = Message::new(&resp);
                request_reply::reply(&message, reply)?;
            }
            _ => {
                return Ok(());
            }
        }
        Ok(())
    }

    // Subscribe to topics.
    #[wasi_otel::instrument(name = "messaging_guest_configure",level = Level::DEBUG)]
    async fn configure() -> Result<Configuration, Error> {
        Ok(Configuration {
            topics: vec!["a".to_string()],
        })
    }
}

wasi_messaging::export!(RequestReply with_types_in wasi_messaging);
