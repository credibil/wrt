# OpenTelemetry Example

This example implements opentelemetry for wasm32 guests using `wasi-otel`. 

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example otel --target wasm32-wasip2
```

### Run using Cargo

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./docker/otelcol.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --features http-server -- run ./target/wasm32-wasip2/debug/examples/otel.wasm
```

### Run using Docker Compose

Docker Compose provides an easy way to run the example with all dependencies.

```bash
docker compose --file ./examples/otel/otel.yaml up
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
