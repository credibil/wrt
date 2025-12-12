use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::anyhow;
use futures::Stream;
use futures::future::FutureExt;
use futures::stream::StreamExt;
use futures::task::{Context, Poll};
use rdkafka::message::{Header, Headers, OwnedHeaders, OwnedMessage};
use rdkafka::producer::BaseRecord;
use rdkafka::{Message as _, Timestamp};
use tokio::sync::mpsc;
use wasi_messaging::{
    Client, FutureResult, Message, MessageProxy, Metadata, RequestOptions, Subscriptions,
    WasiMessagingCtx,
};

const CAPACITY: usize = 1024;

impl WasiMessagingCtx for crate::Client {
    fn connect(&self) -> FutureResult<Arc<dyn Client>> {
        let client = self.clone();
        async move { Ok(Arc::new(client) as Arc<dyn Client>) }.boxed()
    }

    fn new_message(&self, data: Vec<u8>) -> anyhow::Result<Arc<dyn Message>> {
        let now = Timestamp::CreateTime(chrono::Utc::now().timestamp_millis());

        let msg = OwnedMessage::new(
            Some(data),    // payload
            None,          // key
            String::new(), // topic
            now,           // timestamp,
            0,             // partition
            0,             // offset
            None,          // headers
        );
        Ok(Arc::new(KafkaMessage(msg)) as Arc<dyn Message>)
    }

    fn set_content_type(
        &self, message: Arc<dyn Message>, content_type: String,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let kafka_msg = message
            .as_any()
            .downcast_ref::<KafkaMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let msg = kafka_msg.0.clone();
        let headers = kafka_msg.0.headers().cloned().unwrap_or_default();
        let content_header = Header {
            key: "content-type",
            value: Some(content_type.as_bytes()),
        };
        let headers = headers.insert(content_header);
        let msg = msg.replace_headers(Some(headers));
        Ok(Arc::new(KafkaMessage(msg)) as Arc<dyn Message>)
    }

    fn set_payload(
        &self, message: Arc<dyn Message>, data: Vec<u8>,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let kafka_msg = message
            .as_any()
            .downcast_ref::<KafkaMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let msg = kafka_msg.0.clone();
        let msg = msg.set_payload(Some(data));
        Ok(Arc::new(KafkaMessage(msg)) as Arc<dyn Message>)
    }

    fn add_metadata(
        &self, message: Arc<dyn Message>, key: String, value: String,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let kafka_msg = message
            .as_any()
            .downcast_ref::<KafkaMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let msg = kafka_msg.0.clone();
        let headers = kafka_msg.0.headers().cloned().unwrap_or_default();
        let new_header = Header {
            key: &key,
            value: Some(value.as_bytes()),
        };
        let headers = headers.insert(new_header);
        let msg = msg.replace_headers(Some(headers));
        Ok(Arc::new(KafkaMessage(msg)) as Arc<dyn Message>)
    }

    fn set_metadata(
        &self, message: Arc<dyn Message>, metadata: Metadata,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let kafka_msg = message
            .as_any()
            .downcast_ref::<KafkaMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let msg = kafka_msg.0.clone();
        let mut headers = OwnedHeaders::new();
        for (k, v) in &metadata.inner {
            let header = Header {
                key: k,
                value: Some(v.as_bytes()),
            };
            headers = headers.insert(header);
        }
        let msg = msg.replace_headers(Some(headers));
        Ok(Arc::new(KafkaMessage(msg)) as Arc<dyn Message>)
    }

    fn remove_metadata(
        &self, message: Arc<dyn Message>, key: String,
    ) -> anyhow::Result<Arc<dyn Message>> {
        let kafka_msg = message
            .as_any()
            .downcast_ref::<KafkaMessage>()
            .ok_or_else(|| anyhow!("invalid message type"))?;
        let msg = kafka_msg.0.clone();
        let headers = kafka_msg.0.headers().cloned().unwrap_or_default();
        let mut new_headers = OwnedHeaders::new();
        for h in headers.iter() {
            if h.key != key {
                new_headers = new_headers.insert(h.clone());
            }
        }
        let msg = msg.replace_headers(Some(new_headers));
        Ok(Arc::new(KafkaMessage(msg)) as Arc<dyn Message>)
    }
}

#[derive(Debug)]
struct KafkaMessage(OwnedMessage);

impl Message for KafkaMessage {
    fn topic(&self) -> String {
        self.0.topic().to_string()
    }

    fn payload(&self) -> Vec<u8> {
        self.0.payload().unwrap_or_default().to_vec()
    }

    fn metadata(&self) -> Option<Metadata> {
        self.0.headers().map(|headers| {
            let mut md = HashMap::new();
            for h in headers.iter() {
                let bytes = h.value.unwrap_or_default();
                let v = String::from_utf8_lossy(bytes).to_string();
                md.insert(h.key.to_string(), v);
            }
            Metadata { inner: md }
        })
    }

    fn description(&self) -> Option<String> {
        if let Some(headers) = self.0.headers() {
            for h in headers.iter() {
                if h.key == "description"
                    && let Some(bytes) = h.value
                {
                    return Some(String::from_utf8_lossy(bytes).to_string());
                }
            }
        }
        None
    }

    fn length(&self) -> usize {
        self.0.payload().map_or(0, <[u8]>::len)
    }

    fn reply(&self) -> Option<wasi_messaging::Reply> {
        None
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Client for crate::Client {
    fn subscribe(&self) -> FutureResult<Subscriptions> {
        let client = self.clone();

        async move {
            let Some(consumer) = client.consumer else {
                return Err(anyhow!("No topics specified"));
            };
            let registry = client.registry;

            // spawn a task to read messages and forward subscriber
            let (sender, receiver) = mpsc::channel::<MessageProxy>(CAPACITY);
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
                            let decoded_payload = if let Some(sr) = &registry {
                                let topic = msg.topic();
                                let payload_bytes = msg.payload().unwrap_or_default().to_vec();
                                sr.validate_and_decode_json(topic, &payload_bytes).await
                            } else {
                                msg.payload().unwrap_or_default().to_vec()
                            };
                            let message = MessageProxy(Arc::new(KafkaMessage(
                                msg.detach().set_payload(Some(decoded_payload)),
                            ))
                                as Arc<dyn Message>);
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

    fn send(&self, topic: String, message: MessageProxy) -> FutureResult<()> {
        let client = self.clone();

        // TODO: add offset to header??

        async move {
            // schema registry validation when available
            let payload = if let Some(sr) = &client.registry {
                sr.validate_and_encode_json(&topic, message.payload()).await
            } else {
                message.payload()
            };

            let metadata = message.metadata().unwrap_or_default();
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
        &self, _topic: String, _message: MessageProxy, _options: Option<RequestOptions>,
    ) -> FutureResult<MessageProxy> {
        async move { unimplemented!() }.boxed()
    }
}

#[derive(Debug)]
pub struct Subscriber {
    receiver: mpsc::Receiver<MessageProxy>,
}

impl Stream for Subscriber {
    type Item = MessageProxy;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

// async fn into_message(kafka_msg: &BorrowedMessage<'_>, registry: Option<&Registry>) -> Message {
//     let metadata = kafka_msg.headers().map(|headers| {
//         let mut md = HashMap::new();
//         for h in headers.iter() {
//             let bytes = h.value.unwrap_or_default();
//             let v = String::from_utf8_lossy(bytes).to_string();
//             md.insert(h.key.to_string(), v);
//         }
//         Metadata { inner: md }
//     });

//     let topic = kafka_msg.topic();
//     let payload_bytes = kafka_msg.payload().unwrap_or_default().to_vec();

//     let payload = if let Some(sr) = &registry {
//         sr.validate_and_decode_json(topic, &payload_bytes).await
//     } else {
//         payload_bytes
//     };

//     let length = payload.len();

//     Message {
//         topic: topic.to_string(),
//         payload,
//         metadata,
//         length,
//         ..Message::default()
//     }
// }
