//! Kafka client
#![cfg(not(target_arch = "wasm32"))]

mod messaging;
// Custom partitioner for a Kafka producer
mod partitioner;
// Schema registry for Kafka message schema validation
mod schema_registry;

use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::pin::Pin;

use anyhow::Result;
use rdkafka::config::ClientConfig;
use runtime::ResourceBuilder;

use crate::partitioner::Partitioner;
use crate::schema_registry::{SRClient, SchemaConfig};

const DEF_KAFKA_BROKERS: &str = "localhost:9094";
const CLIENT_NAME: &str = "kafka";

/// Kafka resource builder
pub struct Kafka {
    attributes: HashMap<String, String>,
}

impl IntoFuture for Kafka {
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
    type Output = Result<KafkaClient>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.connect())
    }
}

/// Kafka client
#[derive(Clone)]
pub struct KafkaClient {
    config: ClientConfig,
    sr_client: Option<SRClient>,
    partitioner: Option<Partitioner>,
}

impl Debug for KafkaClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KafkaClient")
            .field("config", &self.config)
            .field("sr_client", &self.sr_client.as_ref().map(|_| "Some(SRClient)"))
            .field("partitioner", &self.partitioner.as_ref().map(|_| "Some(Partitioner)"))
            .finish()
    }
}

impl ResourceBuilder<KafkaClient> for Kafka {
    fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    async fn connect(self) -> Result<KafkaClient> {
        let config = config_options();
        let sr_client = schema_registry_options();
        let partitioner = partitioner_options();
        tracing::info!("kafka configured");

        Ok(KafkaClient {
            config,
            sr_client,
            partitioner,
        })
    }
}

fn config_options() -> ClientConfig {
    let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| DEF_KAFKA_BROKERS.into());
    let username = env::var("KAFKA_USERNAME").ok();
    let password = env::var("KAFKA_PASSWORD").ok();
    let consumer_group = env::var("KAFKA_CONSUMER_GROUP").ok();

    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", &brokers);
    if let (Some(user), Some(pass)) = (username, password) {
        config.set("security.protocol", "SASL_SSL");
        config.set("sasl.mechanisms", "PLAIN");
        config.set("sasl.username", &user);
        config.set("sasl.password", &pass);
    }
    if let Some(group_id) = consumer_group {
        config.set("group.id", &group_id);
    }

    config
}

fn schema_registry_options() -> Option<SRClient> {
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
    // Initialize schema registry client if config is provided
    schema_config.as_ref().map_or_else(
        || None,
        |cfg| if cfg.url.is_empty() { None } else { Some(SRClient::new(&cfg.clone())) },
    )
}

fn partitioner_options() -> Option<Partitioner> {
    let js_partitioner = env::var("KAFKA_JS_PARTITIONER").ok().and_then(|s| s.parse::<bool>().ok());
    let partition_count =
        env::var("KAFKA_PARTITION_COUNT").ok().and_then(|s| s.parse::<i32>().ok());
    if js_partitioner.unwrap_or(false) { partition_count.map(Partitioner::new) } else { None }
}
