# Websockets Server Example

This example implements a simple websockets server using `wasi-websockets`.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example websockets --target wasm32-wasip2
```

### Run

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./docker/otelcol.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --bin websockets-runtime --features http,otel,websockets -- run ./target/wasm32-wasip2/debug/examples/websockets.wasm
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
