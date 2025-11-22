#![cfg(target_arch = "wasm32")]

use std::thread::sleep;
use std::time::{Duration, Instant};

use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::{Value, json};
use wasi_http::Result;
use wasi_messaging::types::{Client, Error, Message};
use wasi_messaging::{producer, request_reply};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new()
            .route("/pub-sub", post(pub_sub))
            .route("/request-reply", post(request_reply));
        wasi_http::serve(router, request).await
    }
}

async fn pub_sub(Json(body): Json<Value>) -> Result<Json<Value>> {
    let client = Client::connect("kafka").unwrap();

    let message = Message::new(&Bytes::from(body.to_string()));
    message.set_content_type("application/json");
    message.add_metadata("key", "example_key");

    // TODO: really want spawn here but handler returns and the guest is
    // dropped before the async task completes. Needs investigation.
    wit_bindgen::block_on(async move {
        if let Err(e) = producer::send(&client, "a".to_string(), message).await {
            tracing::error!("error sending message to topic 'a': {e}");
        }
        println!("handler: message published to topic 'a'");
    });

    Ok(Json(json!({"message": "message published"})))
}

async fn request_reply(body: Bytes) -> Json<Value> {
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

pub struct Messaging;
wasi_messaging::export!(Messaging with_types_in wasi_messaging);

impl wasi_messaging::incoming_handler::Guest for Messaging {
    async fn handle(message: Message) -> anyhow::Result<(), Error> {
        tracing::debug!("start processing msg");

        let topic = message.topic().unwrap_or_default();
        tracing::debug!("message received for: {topic}");

        let data = message.data();
        let msg = String::from_utf8(data).unwrap_or_default();

        match topic.as_str() {
            "a" => {
                tracing::debug!("handling topic a");

                // send message to topic `b`
                let mut resp = b"topic a says: ".to_vec();
                resp.extend(message.data());

                let pubmsg = Message::new(&resp);
                if let Some(md) = message.metadata() {
                    pubmsg.set_metadata(&md);
                }
                if let Some(format) = message.content_type() {
                    pubmsg.set_content_type(&format);
                }

                let timer = Instant::now();

                // use `spawn` to avoid blocking for non-blocking execution
                for i in 0..1000 {
                    wit_bindgen::spawn(async move {
                        tracing::debug!("sending message iteration {i}");
                        let Ok(client) = Client::connect("kafka") else {
                            tracing::error!("failed to connect kafka client");
                            return;
                        };

                        let data = format!("topic a iteration {i}");
                        let message = Message::new(data.as_bytes());
                        message.add_metadata("key", &format!("key-{i}"));

                        if let Err(e) = producer::send(&client, "b".to_string(), message).await {
                            tracing::error!("error sending message to topic 'b': {e}");
                        }
                        tracing::debug!("message iteration {i} sent");

                        // HACK: yield to host
                        if i % 100 == 0 {
                            // sleep(Duration::from_nanos(1));
                            wit_bindgen::yield_async().await;
                        }
                    });
                }
                println!("sent 100 messages in {} milliseconds", timer.elapsed().as_millis());
            }
            "b" => {
                tracing::debug!("handling topic b");
            }
            "c" => {
                let data = message.data();
                let data_str = String::from_utf8(data.clone())
                    .map_err(|e| Error::Other(format!("not utf8: {e}")))?;
                tracing::debug!("message received on topic 'c': {data_str}");

                // reply
                let mut resp = b"Hello from topic c: ".to_vec();
                resp.extend(data);

                let reply = Message::new(&resp);
                request_reply::reply(&message, reply)?;
            }
            _ => {
                tracing::debug!("unknown topic: {topic}");
            }
        }

        tracing::debug!("finished processing msg");
        Ok(())
    }
}
