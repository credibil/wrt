#![cfg(not(target_arch = "wasm32"))]

//! Kafka Client.

mod messaging;
mod partitioner;
mod registry;

use std::env;
use std::fmt::{self, Debug};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use fromenv::{FromEnv, ParseResult};
use rdkafka::consumer::StreamConsumer;
use rdkafka::producer::{DeliveryResult, ProducerContext, ThreadedProducer};
use rdkafka::{ClientConfig, ClientContext, Message as _};
use runtime::Resource;
use tracing::instrument;

use crate::partitioner::Partitioner;
use crate::registry::Registry;

#[derive(Clone)]
pub struct Client {
    producer: ThreadedProducer<Tracer>,
    consumer: Arc<StreamConsumer>,
    partitioner: Option<Partitioner>,
    registry: Option<Registry>,
    topics: Vec<String>,
}

impl Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KafkaClient").finish()
    }
}

impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let client_config = ClientConfig::from(&options);

        // maybe custom partitioner
        let partitioner = if options.js_partitioner.unwrap_or_default() {
            options.partition_count.map(Partitioner::new)
        } else {
            None
        };

        // maybe schema registry
        let registry = if let Some(cfg) = options.schema.as_ref()
            && !cfg.url.is_empty()
        {
            Some(Registry::new(cfg))
        } else {
            None
        };

        // producer and consumer
        let producer = client_config
            .create_with_context(Tracer {})
            .map_err(|e| anyhow!("issue creating producer: {e}"))?;
        let consumer =
            client_config.create().map_err(|e| anyhow!("issue creating consumer: {e}"))?;

        Ok(Self {
            producer,
            consumer: Arc::new(consumer),
            partitioner,
            registry,
            topics: options.topics,
        })
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "KAFKA_BROKERS", default = "localhost:9094")]
    pub brokers: String,
    #[env(from = "KAFKA_TOPICS", with = split)]
    pub topics: Vec<String>,
    #[env(from = "KAFKA_CONSUMER_GROUP")]
    pub group_id: Option<String>,
    #[env(from = "KAFKA_USERNAME")]
    pub username: Option<String>,
    #[env(from = "KAFKA_PASSWORD")]
    pub password: Option<String>,
    #[env(from = "KAFKA_JS_PARTITIONER")]
    pub js_partitioner: Option<bool>,
    #[env(from = "KAFKA_PARTITION_COUNT")]
    pub partition_count: Option<i32>,
    #[env(nested)]
    pub schema: Option<SchemaConfig>,
}

#[derive(Debug, Clone, FromEnv)]
pub struct SchemaConfig {
    #[env(from = "SCHEMA_REGISTRY_URL", default = "")]
    pub url: String,
    #[env(from = "SCHEMA_REGISTRY_API_KEY")]
    api_key: Option<String>,
    #[env(from = "SCHEMA_REGISTRY_API_SECRET")]
    api_secret: Option<String>,
    #[env(from = "SCHEMA_CACHE_TTL_SECS")]
    cache_ttl_secs: Option<u64>,
}

#[allow(clippy::unnecessary_wraps)]
fn split(s: &str) -> ParseResult<Vec<String>> {
    Ok(s.split(',').map(ToOwned::to_owned).collect())
}

impl From<&ConnectOptions> for ClientConfig {
    fn from(kafka: &ConnectOptions) -> Self {
        let mut config = Self::new();
        config.set("bootstrap.servers", kafka.brokers.clone());

        // SASL authentication
        if let Some(user) = kafka.username.as_ref()
            && let Some(pass) = kafka.password.as_ref()
        {
            config.set("security.protocol", "SASL_SSL");
            config.set("sasl.mechanisms", "PLAIN");
            config.set("sasl.username", user);
            config.set("sasl.password", pass);
        }

        if let Some(group_id) = kafka.group_id.clone() {
            config.set("group.id", &group_id);
        }

        config
    }
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}

pub struct Tracer;
impl ClientContext for Tracer {}
impl ProducerContext for Tracer {
    type DeliveryOpaque = ();

    fn delivery(&self, delivery_result: &DeliveryResult<'_>, (): Self::DeliveryOpaque) {
        match delivery_result {
            Ok(msg) => {
                let key: &str = msg.key_view().unwrap().unwrap();
                tracing::debug!(
                    "sent message {key} in offset {offset} of partition {partition}",
                    offset = msg.offset(),
                    partition = msg.partition()
                );
            }
            Err((err, message)) => {
                let key: &str = message.key_view().unwrap().unwrap();
                tracing::error!("Failed to send message {key}: {err}");
            }
        }
    }
}
