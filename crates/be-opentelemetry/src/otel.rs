//! OpenTelemetry implementation for the OpenTelemetry gRPC resource.

use futures::FutureExt;
use kernel::FutureResult;
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use wasi_otel::WasiOtelCtx;

use crate::Client;

impl WasiOtelCtx for Client {
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
