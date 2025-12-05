use anyhow::{Context, Result, anyhow};
use fromenv::FromEnv;
use futures::FutureExt;
use kernel::Backend;
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use opentelemetry_proto::tonic::collector::metrics::v1::metrics_service_client::MetricsServiceClient;
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use opentelemetry_proto::tonic::collector::trace::v1::trace_service_client::TraceServiceClient;
use tonic::transport::Channel;
use tracing::instrument;

use crate::host::{FutureResult, WasiOtelCtx};

#[derive(Debug, Clone, Default, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "OTEL_GRPC_URL", default = "http://localhost:4317")]
    pub grpc_url: String,
}

impl kernel::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}

#[derive(Debug, Clone, Default)]
pub struct WasiOtelCtxImpl {
    traces_client: Option<TraceServiceClient<Channel>>,
    metrics_client: Option<MetricsServiceClient<Channel>>,
}

impl Backend for WasiOtelCtxImpl {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let options =
            ConnectOptions::from_env().finalize().context("loading connection options")?;
        tracing::debug!("connecting to OpenTelemetry gRPC endpoint at: {}", options.grpc_url);

        let channel = Channel::from_shared(options.grpc_url)?.connect().await?;
        let traces_client = TraceServiceClient::new(channel.clone());
        let metrics_client = MetricsServiceClient::new(channel);

        Ok(Self {
            traces_client: Some(traces_client),
            metrics_client: Some(metrics_client),
        })
    }
}

impl WasiOtelCtx for WasiOtelCtxImpl {
    /// Export traces using gRPC.
    ///
    /// Errors are logged but not propagated to prevent telemetry failures
    /// from affecting application logic.
    fn export_traces(&self, request: ExportTraceServiceRequest) -> FutureResult<()> {
        let Some(mut client) = self.traces_client.clone() else {
            tracing::error!("no traces client available");
            return Box::pin(async { Ok(()) });
        };

        async move {
            if let Err(e) = client.export(request).await {
                tracing::error!("failed to send traces via gRPC: {e}");
            }
            Ok(())
        }
        .boxed()
    }

    /// Export metrics using gRPC.
    ///
    /// Errors are logged but not propagated to prevent telemetry failures
    /// from affecting application logic.
    fn export_metrics(&self, request: ExportMetricsServiceRequest) -> FutureResult<()> {
        let Some(mut client) = self.metrics_client.clone() else {
            tracing::error!("no metrics client available");
            return Box::pin(async { Ok(()) });
        };

        async move {
            if let Err(e) = client.export(request).await {
                tracing::error!("failed to send metrics via gRPC: {e}");
            }
            Ok(())
        }
        .boxed()
    }
}
