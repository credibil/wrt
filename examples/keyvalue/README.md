# Key-Value Example

This example implements a simple key-value store using `wasi-keyvalue` backed by either Redis or
NATS JetStream.

## Quick Start

To get started add a `.env` file to the workspace root. See `.env.example` for a template.

#### Build

```bash
cargo build --example keyvalue --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a

# NATS
cargo run --features http,otel,keyvalue,nats -- run ./target/wasm32-wasip2/release/examples/keyvalue.wasm

# Redis
cargo run --features http,otel,keyvalue,redis -- run ./target/wasm32-wasip2/release/examples/keyvalue.wasm
```

Alternatively, using Docker Compose:

```bash
# NATS
docker compose --file ./examples/keyvalue/nats.yaml up

# Redis
docker compose --file ./examples/keyvalue/redis.yaml up
```

#### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```