use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use futures::Stream;
use futures::future::FutureExt;
use futures::stream::{self, StreamExt};
use rdkafka::consumer::{Consumer, DefaultConsumerContext, MessageStream, StreamConsumer};
use rdkafka::message::OwnedMessage;
use rdkafka::producer::{BaseRecord, ProducerContext, ThreadedProducer};
use rdkafka::{ClientContext, Message as _};
use wasi_messaging::{
    Client, FutureResult, Message, Metadata, Reply, RequestOptions, Subscriptions,
};

use crate::Client as Kafka;
use crate::partitioner::Partitioner;
use crate::registry::SRClient;

impl Client for Kafka {
    fn subscribe(&self, topics: Vec<String>) -> FutureResult<Subscriptions> {
        let client = self.clone();
        // async move {
        //     let topics = topics.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        //     client.consumer.subscribe(&topics)?;
        //     let stream: MessageStream<'_, DefaultConsumerContext> = client.consumer.stream();

        //     let stream = stream.map(|msg| {
        //         let msg = msg.unwrap();
        //         let mut owned_msg = msg.detach();
        //         Message::new()
        //     });

        //     Ok(Box::pin(stream) as Subscriptions)
        // }
        // .boxed()

        todo!()
    }

    // fn subscribe2(&self, topics: Vec<String>) -> anyhow::Result<Subscription> {
    //     let client = self.clone();

    //     let topics = topics.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    //     client.consumer.subscribe(&topics)?;
    //     let stream: MessageStream<'_, DefaultConsumerContext> = client.consumer.stream();

    //     let stream = stream.map(|msg| {
    //         let msg = msg.unwrap();
    //         let mut owned_msg = msg.detach();
    //         Message::new()
    //     });

    //     Ok(Arc::new(stream))
    // }

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

            let key = metadata.get("key").cloned().unwrap_or_default();
            let mut record = BaseRecord::to(&topic).payload(&payload).key(key.as_bytes());

            // custom partitioning when available AND message doesn't specify partition
            let partition = metadata.get("partition").cloned().unwrap_or_default();
            let partition: i32 = partition.parse().unwrap_or(-1);

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

// let now = i64::try_from(
//     SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis(),
// ); // rdkafka expects i64
//
// let msg = Message::new(
//     data.into(),                 //payload
//     None,                        //key
//     String::new(),               //topic
//     Timestamp::CreateTime(now?), //timestamp
//     -1,                          //partition
//     -1,                          //offset
//     None,                        //headers
// );

// /// Helper: build a new message based on an existing one, overriding only some fields.
// pub fn build_message(
//     base: &Message, payload: Option<Vec<u8>>, headers: Option<OwnedHeaders>,
// ) -> Message {
//     let new_headers = headers.or_else(|| base.headers().cloned());
//     let new_key = new_headers
//         .as_ref()
//         .and_then(|hs| hs.iter().find(|h| h.key == "key"))
//         .and_then(|h| h.value)
//         .map(<[u8]>::to_vec);
//     Message::new(
//         payload.or_else(|| base.payload().map(<[u8]>::to_vec)),
//         new_key,
//         base.topic().to_string(),
//         base.timestamp(),
//         base.partition(),
//         base.offset(),
//         new_headers,
//     )
// }

// fn into_message(kafka_msg: OwnedMessage) -> Message {
//     let metadata = kafka_msg.headers().map(|headers| {
//         let mut header_map = HashMap::new();
//         for (k, v) in headers.iter() {
//             let v = v.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
//             header_map.insert(k.to_string(), v);
//         }
//         Metadata { inner: header_map }
//     });

//     let reply = kafka_msg.reply.map(|reply| Reply {
//         client_name: String::new(),
//         topic: reply.to_string(),
//     });

//     Message {
//         topic: kafka_msg.subject.to_string(),
//         payload: nats_msg.payload.to_vec(),
//         metadata,
//         description: None,
//         length: nats_msg.payload.len(),
//         reply,
//     }
// }

/// Kafka producer client
pub struct KafkaProducer {
    pub producer: ThreadedProducer<ProduceCallbackLogger>,
    pub partitioner: Option<Partitioner>,
    pub sr_client: Option<SRClient>,
}

/// Logger for Kafka produce callbacks
pub struct ProduceCallbackLogger;

impl ClientContext for ProduceCallbackLogger {}

impl ProducerContext for ProduceCallbackLogger {
    type DeliveryOpaque = ();

    fn delivery(
        &self, delivery_result: &rdkafka::producer::DeliveryResult<'_>,
        _delivery_opaque: Self::DeliveryOpaque,
    ) {
        let dr = delivery_result.as_ref();
        //let msg = dr.unwrap();

        match dr {
            Ok(msg) => {
                let key: &str = msg.key_view().unwrap().unwrap();
                tracing::debug!(
                    "produced message with key {} in offset {} of partition {}",
                    key,
                    msg.offset(),
                    msg.partition()
                );
            }
            Err((producer_err, message)) => {
                let key: &str = message.key_view().unwrap().unwrap();

                // Log or forward the structured error
                tracing::error!("Failed to produce message with key '{}': {}", key, producer_err);
            }
        }
    }
}
