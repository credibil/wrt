#![cfg(not(target_arch = "wasm32"))]

//! Kafka Client.

mod messaging;
mod partitioner;
mod registry;

use std::env;
use std::fmt::{self, Debug};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use rdkafka::consumer::StreamConsumer;
use rdkafka::producer::ThreadedProducer;
use rdkafka::{ClientConfig, Message as _, Timestamp};
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
        let server_config = server_config();

        //--------------------------------------------------
        // Client Configuration
        //--------------------------------------------------
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", server_config.brokers.clone());

        // SASL authentication
        if let Some(user) = server_config.username.as_ref()
            && let Some(pass) = server_config.password.as_ref()
        {
            client_config.set("security.protocol", "SASL_SSL");
            client_config.set("sasl.mechanisms", "PLAIN");
            client_config.set("sasl.username", user);
            client_config.set("sasl.password", pass);
        }

        // initialize custom partitioner when `js_partitioner` is true
        let partitioner = if server_config.js_partitioner.unwrap_or_default() {
            server_config.partition_count.map(Partitioner::new)
        } else {
            None
        };

        // initialize schema registry client when `client_config` is provided
        let registry = if let Some(cfg) = server_config.schema.as_ref()
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
            // client_config,
        })
    }
}

fn server_config() -> ServerConfig {
    let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| DEF_KAFKA_BROKERS.into());
    let username = env::var("KAFKA_USERNAME").ok();
    let password = env::var("KAFKA_PASSWORD").ok();
    let consumer_group = env::var("KAFKA_CONSUMER_GROUP").ok();
    let js_partitioner = env::var("KAFKA_JS_PARTITIONER").ok().and_then(|s| s.parse::<bool>().ok());
    let partition_count =
        env::var("KAFKA_PARTITION_COUNT").ok().and_then(|s| s.parse::<i32>().ok());

    let schema_config = env::var("SCHEMA_REGISTRY_URL").map_or(None, |url| {
        let api_key = env::var("SCHEMA_REGISTRY_API_KEY").ok();
        let api_secret = env::var("SCHEMA_REGISTRY_API_SECRET").ok();
        let cache_ttl_secs =
            env::var("SCHEMA_CACHE_TTL_SECS").ok().and_then(|v| v.parse::<u64>().ok());

        Some(SchemaConfig {
            url,
            api_key,
            api_secret,
            cache_ttl_secs,
        })
    });

    tracing::info!("Kafka configuration built for brokers: {brokers}");

    ServerConfig {
        brokers: brokers.clone(),
        username,
        password,
        group_id: consumer_group,
        js_partitioner,
        partition_count,
        schema: schema_config,
    }
}

/// Kafka configuration
#[derive(Debug, Clone)]
struct ServerConfig {
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
