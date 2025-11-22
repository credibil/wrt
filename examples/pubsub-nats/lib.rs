#![cfg(target_arch = "wasm32")]

use std::thread::sleep;
use std::time::{Duration, Instant};

use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::{Value, json};
use wasi_http::Result;
use wasi_messaging::producer;
use wasi_messaging::types::{Client, Error, Message};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", post(handler));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
async fn handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    let body_bytes = Bytes::from(body.to_string());
    let client = Client::connect("nats").unwrap();
    let message = Message::new(&body_bytes);

    // TODO: really want spawn here but handler returns and the guest is
    // dropped before the async task completes. Needs investigation.
    wit_bindgen::block_on(async move {
        if let Err(e) = producer::send(&client, "a".to_string(), message).await {
            tracing::error!("error sending message to topic 'a': {e}");
        }
    });

    Ok(Json(json!({"message": "message published"})))
}

pub struct Messaging;
wasi_messaging::export!(Messaging with_types_in wasi_messaging);

impl wasi_messaging::incoming_handler::Guest for Messaging {
    async fn handle(message: Message) -> Result<(), Error> {
        let data = message.data();
        let data_str =
            String::from_utf8(data.clone()).map_err(|e| Error::Other(format!("not utf8: {e}")))?;

        match message.topic().as_deref() {
            Some("a") => {
                tracing::debug!("message received with topic 'a': {data_str}");

                // send message to topic `b`
                let mut resp = b"topic a says: ".to_vec();
                resp.extend(data);

                let message = Message::new(&resp);
                if let Some(md) = message.metadata() {
                    message.set_metadata(&md);
                }

                // set `content_type` *after* `metadata` otherwise it is overwritten
                if let Some(format) = message.content_type() {
                    message.set_content_type(&format);
                }

                let timer = Instant::now();

                // *** WASIP3 ***
                // use `spawn` to avoid blocking for non-blocking execution
                for i in 0..100 {
                    wit_bindgen::spawn(async move {
                        let client = Client::connect("nats").unwrap();
                        let data = format!("topic a iteration {i}");
                        let message = Message::new(data.as_bytes());

                        if let Err(e) = producer::send(&client, "b".to_string(), message).await {
                            tracing::error!("error sending message to topic 'b': {e}");
                        }

                        // HACK: yield to host
                        if i % 100 == 0 {
                            sleep(Duration::from_nanos(1));
                            // wit_bindgen::yield_async().await;
                        }
                    });
                }

                println!("sent 100 messages in {} milliseconds", timer.elapsed().as_millis());
            }
            Some("b") => {
                tracing::debug!("message received on topic 'b': {data_str}");
            }
            _ => {
                return Ok(());
            }
        }
        Ok(())
    }
}
