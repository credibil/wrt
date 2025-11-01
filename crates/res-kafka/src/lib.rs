#![cfg(not(target_arch = "wasm32"))]

//! Kafka Client.

mod messaging;
mod partitioner;
mod registry;

use std::env;
use std::fmt::{self, Debug};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use rdkafka::ClientConfig;
use rdkafka::consumer::StreamConsumer;
use rdkafka::producer::ThreadedProducer;
use runtime::Resource;
use tracing::instrument;

use crate::messaging::ProduceCallbackLogger;
use crate::partitioner::Partitioner;
use crate::registry::SRClient;

const DEF_KAFKA_BROKERS: &str = "localhost:9094";

#[derive(Clone)]
pub struct Client {
    producer: ThreadedProducer<ProduceCallbackLogger>,
    consumer: Arc<StreamConsumer>,
    partitioner: Option<Partitioner>,
    registry: Option<SRClient>,
    // client_config: ClientConfig,
}

impl Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KafkaClient").finish()
    }
}

impl Resource for Client {
    #[instrument]
    async fn connect() -> Result<Self> {
        let kafka_config = KafkaConfig::from_env();
        let client_config = ClientConfig::from(&kafka_config);

        // maybe custom partitioner
        let partitioner = if kafka_config.js_partitioner.unwrap_or_default() {
            kafka_config.partition_count.map(Partitioner::new)
        } else {
            None
        };

        // maybe schema registry
        let registry = if let Some(cfg) = kafka_config.schema.as_ref()
            && !cfg.url.is_empty()
        {
            Some(SRClient::new(cfg))
        } else {
            None
        };

        // producer and consumer
        let producer = client_config
            .create_with_context(ProduceCallbackLogger {})
            .map_err(|e| anyhow!("issue creating producer: {e}"))?;
        let consumer = Arc::new(client_config.create().unwrap());

        Ok(Self {
            producer,
            consumer,
            partitioner,
            registry,
        })
    }
}

/// Kafka configuration
#[derive(Debug, Clone)]
struct KafkaConfig {
    /// Comma-separated list of Kafka brokers
    brokers: String,
    /// Optional username for SASL authentication
    username: Option<String>,
    /// Optional password for SASL authentication
    password: Option<String>,
    /// Consumer group ID
    group_id: Option<String>, // only used for consumer
    /// Enable custom JS partitioner for producer
    js_partitioner: Option<bool>,
    /// Number of partitions for custom partitioner
    partition_count: Option<i32>, // only used for producer
    /// Optional schema registry configuration
    schema: Option<SchemaConfig>,
}

impl KafkaConfig {
    fn from_env() -> Self {
        let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| DEF_KAFKA_BROKERS.into());
        let username = env::var("KAFKA_USERNAME").ok();
        let password = env::var("KAFKA_PASSWORD").ok();
        let consumer_group = env::var("KAFKA_CONSUMER_GROUP").ok();
        let js_partitioner = env::var("KAFKA_JS_PARTITIONER").ok().and_then(|s| s.parse().ok());
        let partition_count = env::var("KAFKA_PARTITION_COUNT").ok().and_then(|s| s.parse().ok());
        let schema = SchemaConfig::from_env();

        tracing::info!("Kafka configuration built for brokers: {brokers}");

        Self {
            brokers,
            username,
            password,
            group_id: consumer_group,
            js_partitioner,
            partition_count,
            schema,
        }
    }
}

impl From<&KafkaConfig> for ClientConfig {
    fn from(kafka: &KafkaConfig) -> Self {
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

        config
    }
}

/// Schema registry configuration
#[derive(Debug, Clone)]
struct SchemaConfig {
    /// Schema registry URL
    url: String,
    /// Optional API key for schema registry
    api_key: Option<String>,
    /// Optional API secret for schema registry
    api_secret: Option<String>,
    /// Optional cache TTL in seconds for schema registry
    cache_ttl_secs: Option<u64>,
}

impl SchemaConfig {
    fn from_env() -> Option<Self> {
        let url = env::var("SCHEMA_REGISTRY_URL").ok()?;
        let api_key = env::var("SCHEMA_REGISTRY_API_KEY").ok();
        let api_secret = env::var("SCHEMA_REGISTRY_API_SECRET").ok();
        let cache_ttl_secs = env::var("SCHEMA_CACHE_TTL_SECS").ok().and_then(|v| v.parse().ok());

        Some(Self {
            url,
            api_key,
            api_secret,
            cache_ttl_secs,
        })
    }
}
