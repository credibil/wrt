//! WebAssembly Component Runtime
//!
//! This module provides runtime configuration and utilities for executing
//! WebAssembly components with WASI capabilities. The actual runtime
//! infrastructure (Context, StoreCtx, etc.) is generated using the
//! `codegen::runtime!` macro.
//!
//! ## Usage
//!
//! ```ignore
//! use buildgen::runtime;
//!
//! runtime!({
//!     wasi_http: WasiHttpCtx,
//!     wasi_otel: DefaultOtel,
//!     wasi_blobstore: MongoDb,
//!     wasi_keyvalue: Nats,
//!     wasi_messaging: Nats,
//!     wasi_vault: Azure
//! });
//! ```
//!
//! ## Backend Features
//! - `azure` - Azure services (Key Vault, Identity)
//! - `kafka` - Apache Kafka messaging
//! - `mongodb` - MongoDB blob storage
//! - `nats` - NATS messaging, key-value, and blob storage
//! - `postgres` - `PostgreSQL` database
//! - `redis` - Redis key-value storage
//!
//! ## WASI Interface Features
//! - `blobstore` - Object/blob storage interface
//! - `http` - HTTP client and server interface
//! - `identity` - Identity and authentication interface
//! - `keyvalue` - Key-value storage interface
//! - `messaging` - Pub/sub messaging interface
//! - `otel` - `OpenTelemetry` observability interface
//! - `sql` - SQL database interface
//! - `vault` - Secrets management interface
//! - `websockets` - `WebSocket` interface

#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::doc_markdown)]

pub mod env;

use std::path::Path;

use anyhow::Result;
use fromenv::FromEnv;

/// Runtime configuration loaded from environment variables.
///
/// This configuration is used to customize runtime behavior and is
/// automatically populated from the environment at startup.
#[derive(Debug, Clone, FromEnv)]
pub struct RuntimeConfig {
    /// Deployment environment (e.g., "dev", "staging", "prod").
    #[env(from = "ENV", default = "dev")]
    pub environment: String,

    /// Component name, derived from the wasm filename if not specified.
    #[env(from = "COMPONENT")]
    pub component: Option<String>,

    /// OpenTelemetry collector gRPC endpoint URL.
    #[env(from = "OTEL_GRPC_URL")]
    pub otel_grpc_url: Option<String>,
}

impl RuntimeConfig {
    /// Creates a runtime configuration from a wasm file path.
    ///
    /// Loads configuration from environment variables and derives the
    /// component name from the wasm filename if not already set.
    ///
    /// # Errors
    ///
    /// Returns an error if environment variable parsing fails.
    pub fn from_wasm(wasm: &Path) -> Result<Self> {
        let mut config = Self::from_env().finalize()?;
        let component = wasm.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        // SAFETY: Environment variable modification is safe here because:
        // 1. This runs during single-threaded initialization
        // 2. No other threads have been spawned yet
        // 3. Resource clients that depend on these vars are created after this
        unsafe {
            if std::env::var("COMPONENT").is_err() {
                config.component = Some(component.clone());
                std::env::set_var("COMPONENT", &component);
            }
            #[cfg(feature = "kafka")]
            std::env::set_var("KAFKA_CLIENT_ID", format!("{component}-{}", &config.environment));
        };

        Ok(config)
    }
}

/// Marker trait for runtime state that can be used as a generic bound.
pub trait RuntimeState {}
