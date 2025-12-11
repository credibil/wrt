# OpenTelemetry Example

Demonstrates OpenTelemetry instrumentation for WebAssembly guests using `wasi-otel`.

## Prerequisites

Start the OpenTelemetry Collector:

```bash
docker compose -f docker/otelcol.yaml up -d
```

## Quick Start

```bash
# build the guest
cargo build --example otel-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example otel -- run ./target/wasm32-wasip2/debug/examples/otel_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Cleanup

```bash
docker compose -f docker/otelcol.yaml down -v
```
