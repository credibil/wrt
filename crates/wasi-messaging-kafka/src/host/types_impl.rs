use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use rdkafka::message::{Header, Headers as _, OwnedHeaders};
use rdkafka::{ClientConfig, Message as _, Timestamp};
use wasmtime::component::Resource;

use crate::ProduceCallbackLogger;
pub use crate::host::generated::wasi::messaging::types::Error;
use crate::host::generated::wasi::messaging::types::{self, Client, Message, Metadata, Topic};
use crate::host::server::rebuild_message;
use crate::host::{Host, Result};
use crate::partitioner::Partitioner;
use crate::schema_registry::SRClient;

impl types::Host for Host<'_> {
    fn convert_error(&mut self, err: Error) -> anyhow::Result<Error> {
        Ok(err)
    }
}

impl types::HostClient for Host<'_> {
    async fn connect(&mut self, _name: String) -> Result<Resource<Client>> {
        tracing::debug!("HostClient::connect Kafka");

        let kafka_config = crate::kafka()?;

        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", kafka_config.brokers.clone());

        // Optional SASL authentication
        if let (Some(user), Some(pass)) =
            (kafka_config.username.clone(), kafka_config.password.clone())
        {
            config.set("security.protocol", "SASL_SSL");
            config.set("sasl.mechanisms", "PLAIN");
            config.set("sasl.username", &user);
            config.set("sasl.password", &pass);
        }

        // Optional: Initialize custom partitioner if js_partitioner is true
        let partitioner = if kafka_config.js_partitioner.unwrap_or(false) {
            kafka_config.partition_count.map(Partitioner::new)
        } else {
            None
        };

        // Initialize schema registry client if config is provided
        let sr_client = kafka_config.schema.as_ref().map_or_else(
            || None,
            |cfg| if cfg.url.is_empty() { None } else { Some(SRClient::new(&cfg.clone())) },
        );

        let producer = config
            .create_with_context(ProduceCallbackLogger {})
            .map_err(|e| anyhow!("invalid producer config: {e}"))?;

        Ok(self.table.push(Client {
            producer,
            partitioner,
            sr_client,
        })?)
    }

    async fn disconnect(&mut self, _rep: Resource<Client>) -> Result<()> {
        tracing::debug!("HostClient::disconnect (noop for Kafka producer)");
        Ok(())
    }

    async fn drop(&mut self, rep: Resource<Client>) -> anyhow::Result<()> {
        tracing::debug!("HostClient::drop");
        self.table.delete(rep)?;
        Ok(())
    }
}

impl types::HostMessage for Host<'_> {
    /// Create a new message with the given payload.
    async fn new(&mut self, data: Vec<u8>) -> anyhow::Result<Resource<Message>> {
        tracing::debug!("HostMessage::new with {} bytes", data.len());
        let now = i64::try_from(
            SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis(),
        ); // rdkafka expects i64
        let msg = Message::new(
            data.into(),                 //payload
            None,                        //key
            String::new(),               //topic
            Timestamp::CreateTime(now?), //timestamp
            -1,                          //partition
            -1,                          //offset
            None,                        //headers
        );
        Ok(self.table.push(msg)?)
    }

    /// The topic/subject/channel this message was received on, if any.
    async fn topic(&mut self, self_: Resource<Message>) -> anyhow::Result<Option<Topic>> {
        tracing::debug!("HostMessage::topic");
        let msg = self.table.get(&self_)?;
        let topic = msg.topic();
        if topic.is_empty() { Ok(None) } else { Ok(Some(topic.to_string())) }
    }

    /// An optional content-type describing the format of the data in the
    /// message. This is sometimes described as the "format" type".
    async fn content_type(&mut self, self_: Resource<Message>) -> anyhow::Result<Option<String>> {
        tracing::debug!("HostMessage::content_type");
        let msg = self.table.get(&self_)?;

        // Access headers from the message
        if let Some(headers) = msg.headers() {
            for i in 0..headers.count() {
                let header = headers.get(i); // Header<'_, &[u8]>
                if header.key.eq_ignore_ascii_case("content-type")
                    && let Some(value) = header.value
                {
                    // Convert &[u8] to String
                    return Ok(Some(String::from_utf8_lossy(value).into_owned()));
                }
            }
        }
        Ok(None)
    }

    /// Set the content-type describing the format of the data in the message.
    /// This is sometimes described as the "format" type.
    async fn set_content_type(
        &mut self, self_: Resource<Message>, content_type: String,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::set_content_type {}", content_type);

        let msg = self.table.get_mut(&self_)?;
        let new_headers =
            update_headers(msg.headers(), "content-type", Some(content_type.as_bytes()));
        *msg = rebuild_message(msg, None, Some(new_headers));

        Ok(())
    }

    /// An opaque blob of data.
    async fn data(&mut self, self_: Resource<Message>) -> anyhow::Result<Vec<u8>> {
        tracing::debug!("HostMessage::data");
        let msg = self.table.get(&self_)?;
        let data: Vec<u8> = msg
            .payload()
            .map(<[u8]>::to_vec) // convert &[u8] to Vec<u8>
            .unwrap_or_default(); // default to empty Vec if None

        Ok(data)
    }

    /// Set the opaque blob of data for this message, discarding the old value".
    async fn set_data(&mut self, self_: Resource<Message>, data: Vec<u8>) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::set_data");
        let msg = self.table.get_mut(&self_)?;
        *msg = rebuild_message(msg, Some(data), None);

        Ok(())
    }

    async fn metadata(&mut self, self_: Resource<Message>) -> anyhow::Result<Option<Metadata>> {
        tracing::debug!("HostMessage::metadata");
        let msg = self.table.get(&self_)?;
        let headers = msg.headers().map(|owned| {
            owned
                .iter()
                .map(|h| {
                    (
                        h.key.to_string(),
                        h.value.map(|v| String::from_utf8_lossy(v).to_string()).unwrap_or_default(),
                    )
                })
                .collect()
        });
        Ok(headers)
    }

    async fn add_metadata(
        &mut self, self_: Resource<Message>, key: String, value: String,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::add_metadata {key}={value}");
        let msg = self.table.get_mut(&self_)?;
        let new_headers = update_headers(msg.headers(), &key, Some(value.as_bytes()));

        *msg = rebuild_message(msg, None, Some(new_headers));

        Ok(())
    }

    //Replace all headers with the provided metadata
    async fn set_metadata(
        &mut self, self_: Resource<Message>, meta: Metadata,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::set_metadata");

        let msg = self.table.get_mut(&self_)?;
        let mut new_headers = OwnedHeaders::new();

        // Insert all metadata key/value pairs into headers
        for (key, value) in meta {
            new_headers = new_headers.insert(Header {
                key: &key,
                value: Some(value.as_bytes()),
            });
        }

        // Replace the message headers with the new headers
        *msg = rebuild_message(msg, None, Some(new_headers));

        Ok(())
    }

    async fn remove_metadata(
        &mut self, self_: Resource<Message>, key: String,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::remove_metadata {key}");
        let msg = self.table.get_mut(&self_)?;
        let new_headers = update_headers(msg.headers(), &key, None);

        *msg = rebuild_message(msg, None, Some(new_headers));

        Ok(())
    }

    async fn drop(&mut self, rep: Resource<Message>) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::drop");
        self.table.delete(rep)?;
        Ok(())
    }
}

/// Helper to clone headers and optionally update a single key/value
pub fn update_headers(
    existing: Option<&OwnedHeaders>, key: &str, value: Option<&[u8]>,
) -> OwnedHeaders {
    let mut new_headers = OwnedHeaders::new();
    if let Some(hdrs) = existing {
        for h in hdrs.iter() {
            if !h.key.eq_ignore_ascii_case(key) {
                new_headers = new_headers.insert(h);
            }
        }
    }
    if let Some(val) = value {
        new_headers = new_headers.insert(Header {
            key,
            value: Some(val),
        });
    }
    new_headers
}
