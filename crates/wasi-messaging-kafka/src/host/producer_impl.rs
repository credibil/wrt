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
