# Key-Value Example

This example demonstrates using a key-value store with Redis or NATS.

## Quick Start

To get started add a `.env` file to the workspace root. See `.env.example` for a template.

#### Build

```bash
# NATS
cargo build --example keyvalue --features http,keyvalue,nats --target wasm32-wasip2 --release

# Redis
cargo build --example keyvalue --features http,keyvalue,redis --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a

# NATS
cargo run --features http,keyvalue,nats -- run ./target/wasm32-wasip2/release/examples/keyvalue.wasm

# Redis
cargo run --features http,keyvalue,redis -- run ./target/wasm32-wasip2/release/examples/keyvalue.wasm
```

Docker Compose can also be used to run the service:

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