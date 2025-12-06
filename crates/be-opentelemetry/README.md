# be-opentelemetry

OpenTelemetry gRPC backend implementation for the `wasi-otel` interface.

This backend connects to an OpenTelemetry Collector via gRPC and exports traces and metrics using the OTLP protocol.

## Configuration

The backend is configured through environment variables:

- `OTEL_GRPC_URL` - The gRPC endpoint URL (default: `http://localhost:4317`)

## Example

```rust
use be_opentelemetry::Client;
use kernel::Backend;

let client = Client::connect().await?;
```

## Features

- Exports traces to OpenTelemetry Collector via gRPC
- Exports metrics to OpenTelemetry Collector via gRPC
- Automatic retry and connection management via tonic
- Non-blocking exports (errors are logged but don't fail application logic)

