use anyhow::anyhow;
use rdkafka::Message as _;
use rdkafka::producer::BaseRecord;
use wasmtime::component::{Accessor, Resource};

use crate::host::generated::wasi::messaging::producer;
use crate::host::generated::wasi::messaging::producer::{Client, Message};
use crate::host::generated::wasi::messaging::types::Topic;
use crate::host::{Host, HostData, Result};

// *** WASIP3 ***
// use `HostWithStore` to add async support`

impl producer::Host for Host<'_> {}

/// The producer interface is used to send messages to a channel/topic.
impl producer::HostWithStore for HostData {
    /// Sends the message using the given client.
    async fn send<T>(
        accessor: &Accessor<T, Self>, c: Resource<Client>, topic: Topic, message: Resource<Message>,
    ) -> Result<()> {
        tracing::trace!("producer::Host::send: topic {topic:?}");

        let (producer, msg, partitioner, schema_registry) = accessor.with(move |mut access| {
            let table = access.get().table;
            let kafka_resource = table.get(&c)?;
            let partitioner = kafka_resource.partitioner.clone();
            let schema_registry = kafka_resource.sr_client.clone();
            let msg = table.get(&message)?;
            let producer = kafka_resource.producer.clone();
            Ok::<_, anyhow::Error>((producer, msg.clone(), partitioner, schema_registry))
        })?;

        let Some(producer) = producer else {
            return Err(anyhow!("producer not initialized").into());
        };

        //schea registry validation if provided
        let payload_bytes = msg.payload().unwrap_or(&[]).to_vec();
        let payload = if let Some(sr) = &schema_registry {
            // schema_registry exists → serialize
            sr.validate_and_encode_json(&topic, payload_bytes).await
        } else {
            // no schema_registry → use raw payload
            payload_bytes
        };

        let mut record = BaseRecord::to(&topic).payload(&payload).key(msg.key().unwrap_or(&[]));

        //custom partitioning if provided and message doesn't have specific partition
        let partition = msg.partition();
        if partition >= 0 {
            record = record.partition(partition);
        } else if let Some(partitioner) = partitioner
            && let Some(key) = msg.key()
        {
            let partition = partitioner.partition(key);
            record = record.partition(partition);
        }

        let p: i32 = msg.partition();
        if p >= 0 {
            record = record.partition(p);
        }

        let send_result = producer.send(record);

        match send_result {
            Ok(()) => Ok(()),
            Err((err, _)) => {
                tracing::trace!("producer::error {}", err);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rdkafka::{Timestamp, message::{Header, Headers as _, OwnedHeaders}};
    use wasmtime::component::ResourceTable;

    use crate::host::{server::rebuild_message, types_impl::update_headers};

    use super::*;

    struct DummyHost {
        table: ResourceTable,
    }

    impl DummyHost {
        fn new() -> Self {
            Self {
                table: ResourceTable::default(),
            }
        }
    }

    #[tokio::test]
    async fn test_rebuild_message_preserves_payload_and_topic() {
        let original_payload = b"hello".to_vec();
        let msg = Message::new(
            Some(original_payload.clone()),
            None,
            "topic1".to_string(),
            Timestamp::NotAvailable,
            0,
            0,
            None,
        );

        let rebuilt = rebuild_message(&msg, None, None);

        assert_eq!(rebuilt.payload().unwrap(), original_payload.as_slice());
        assert_eq!(rebuilt.topic(), "topic1");
        assert_eq!(rebuilt.key(), None);
    }

    #[tokio::test]
    async fn test_rebuild_key_from_headers() {
        let headers = OwnedHeaders::new().insert(Header {
            key: "key",
            value: Some(b"foo"),
        });
        let msg =
            Message::new(None, None, "topic1".to_string(), Timestamp::NotAvailable, 0, 0, None);
        let rebuilt = rebuild_message(&msg, None, Some(headers));
        assert_eq!(rebuilt.key().unwrap(), b"foo");

        let headers = OwnedHeaders::new().insert(Header {
            key: "unrelated",
            value: Some(b"foo"),
        });
        let rebuilt = rebuild_message(&msg, None, Some(headers));
        assert_eq!(rebuilt.key(), None);
    }

    #[tokio::test]
    async fn test_update_headers_add_and_override() {
        use rdkafka::message::{Header, OwnedHeaders};

        // Start with one header: content-type=qux
        let headers = OwnedHeaders::new().insert(Header {
            key: "content-type",
            value: Some(b"qux"),
        });

        // Call update_headers to replace "qux" with "new"
        let updated = update_headers(Some(&headers), "content-type", Some(b"new"));

        // Collect all header key/values
        let values: Vec<(String, String)> = updated
            .iter()
            .map(|h| {
                (h.key.to_string(), String::from_utf8_lossy(h.value.unwrap_or(b"")).into_owned())
            })
            .collect();

        assert_eq!(values, vec![("content-type".to_string(), "new".to_string())]);
    }

    #[tokio::test]
    async fn test_hostmessage_set_and_get_data() {
        let mut host = DummyHost::new();

        let data = b"test payload".to_vec();
        let msg = Message::new(
            Some(data),
            None,
            "topic".to_string(),
            Timestamp::NotAvailable,
            -1,
            -1,
            None,
        );

        let res = host.table.push(msg).unwrap();

        // Set new payload
        let new_data = b"new data".to_vec();
        let msg_mut = host.table.get_mut(&res).unwrap();
        *msg_mut = rebuild_message(msg_mut, Some(new_data.clone()), None);

        // Get payload
        let retrieved = host.table.get(&res).unwrap();
        assert_eq!(retrieved.payload().unwrap(), new_data.as_slice());
    }
}
