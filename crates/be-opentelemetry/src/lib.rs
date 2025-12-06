//! OpenTelemetry gRPC Client.
#![allow(missing_docs)]
#![cfg(not(target_arch = "wasm32"))]

mod otel;

use std::fmt::Debug;

use anyhow::{Context, Result};
use fromenv::FromEnv;
use kernel::Backend;
use opentelemetry_proto::tonic::collector::metrics::v1::metrics_service_client::MetricsServiceClient;
use opentelemetry_proto::tonic::collector::trace::v1::trace_service_client::TraceServiceClient;
use tonic::transport::Channel;
use tracing::instrument;

#[derive(Clone)]
pub struct Client {
    traces_client: TraceServiceClient<Channel>,
    metrics_client: MetricsServiceClient<Channel>,
}

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Backend for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument(name = "OpenTelemetry::connect_with")]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        tracing::debug!("connecting to OpenTelemetry gRPC endpoint at: {}", options.grpc_url);

        let channel = Channel::from_shared(options.grpc_url)?
            .connect()
            .await
            .context("failed to connect to OpenTelemetry gRPC endpoint")?;
        
        let traces_client = TraceServiceClient::new(channel.clone());
        let metrics_client = MetricsServiceClient::new(channel);

        tracing::info!("connected to OpenTelemetry gRPC endpoint");
        Ok(Self {
            traces_client,
            metrics_client,
        })
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "OTEL_GRPC_URL", default = "http://localhost:4317")]
    pub grpc_url: String,
}

impl kernel::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().context("issue loading connection options")
    }
}

