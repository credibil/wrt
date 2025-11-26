# Key-Value Example

This example implements a simple key-value store using `wasi-keyvalue` backed by either Redis or
NATS JetStream.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example keyvalue --target wasm32-wasip2
```

### Run using Cargo

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./docker/otelcol.yaml up
```

Run the guest using either NATS or Redis:

```bash
set -a && source .env && set +a

# with NATS
cargo run --features http,otel,keyvalue,nats -- run ./target/wasm32-wasip2/debug/examples/keyvalue.wasm

# with Redis
cargo run --features http,otel,keyvalue,redis -- run ./target/wasm32-wasip2/debug/examples/keyvalue.wasm
```

### Run using Docker Compose

Docker Compose provides an easy way to run the example with all dependencies.

```bash
# with NATS
docker compose --file ./examples/keyvalue/nats.yaml up

# with Redis
docker compose --file ./examples/keyvalue/redis.yaml up
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```