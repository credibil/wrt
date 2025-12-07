# OpenTelemetry Example

This example implements opentelemetry for wasm32 guests using `wasi-otel`.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the guest

```bash
cargo build --example otel-wasm --target wasm32-wasip2
```

### Run

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./docker/otelcol.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --example otel -- run ./target/wasm32-wasip2/debug/examples/otel_wasm.wasm
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
