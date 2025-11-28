use anyhow::{Context, Result, anyhow};
use fromenv::FromEnv;
use futures::FutureExt;
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use opentelemetry_proto::tonic::collector::metrics::v1::metrics_service_client::MetricsServiceClient;
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use opentelemetry_proto::tonic::collector::trace::v1::trace_service_client::TraceServiceClient;
use runtime::Resource;
use tonic::transport::{Channel, Endpoint};
use tracing::instrument;

use crate::host::{FutureResult, WasiOtelCtx};

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "OTEL_GRPC_URL", default = "http://localhost:4317")]
    pub grpc_url: String,
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}

#[derive(Debug, Clone)]
pub struct DefaultOtelCtx {
    traces_client: TraceServiceClient<Channel>,
    metrics_client: MetricsServiceClient<Channel>,
}

impl Resource for DefaultOtelCtx {
    type ConnectOptions = ConnectOptions;

    #[instrument(name = "GrpcOtel::connect_with")]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let options =
            ConnectOptions::from_env().finalize().context("loading connection options")?;
        tracing::debug!("connecting to OpenTelemetry gRPC endpoint at: {}", options.grpc_url);

        let endpoint = Endpoint::from_shared(options.grpc_url)?;
        let traces_client = TraceServiceClient::connect(endpoint.clone()).await?;
        let metrics_client = MetricsServiceClient::connect(endpoint).await?;

        Ok(Self {
            traces_client,
            metrics_client,
        })
    }
}

impl WasiOtelCtx for DefaultOtelCtx {
    /// Export traces using gRPC.
    ///
    /// Errors are logged but not propagated to prevent telemetry failures
    /// from affecting application logic.
    fn export_traces(&self, request: ExportTraceServiceRequest) -> FutureResult<()> {
        let mut client = self.traces_client.clone();

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
        let mut client = self.metrics_client.clone();

        async move {
            if let Err(e) = client.export(request).await {
                tracing::error!("failed to send metrics via gRPC: {e}");
            }
            Ok(())
        }
        .boxed()
    }
}
