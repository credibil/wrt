#![cfg(all(target_arch = "wasm32", not(miri)))]

use anyhow::Result;
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::{Value, json};
use tracing::Level;
use wasi_messaging::request_reply;
use wasi_messaging::types::{Client, Error, Message};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", post(handler));
        wasi_http::serve(router, request).await
    }
}

#[wasi_otel::instrument]
async fn handler(body: Bytes) -> Json<Value> {
    let client = Client::connect("nats").unwrap();
    let message = Message::new(&body);
    let reply = wit_bindgen::block_on(async move {
        request_reply::request(&client, "a".to_string(), &message, None).await
    })
    .expect("should reply");

    // process first reply
    let data = reply[0].data();
    let data_str = String::from_utf8_lossy(&data);

    Json(json!({"reply": data_str}))
}

pub struct RequestReply;
wasi_messaging::export!(RequestReply with_types_in wasi_messaging);

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
}
