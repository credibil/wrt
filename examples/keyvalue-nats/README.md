# Key-Value Example (NATS)

Demonstrates `wasi-keyvalue` backed by NATS JetStream for persistent key-value storage.

## Prerequisites

Start NATS JetStream:

```bash
docker compose -f examples/keyvalue-nats/nats.yaml up -d
```

## Quick Start

```bash
# build the guest
cargo build --example keyvalue-nats-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example keyvalue-nats -- run ./target/wasm32-wasip2/debug/examples/keyvalue_nats_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Cleanup

```bash
docker compose -f examples/keyvalue-nats/nats.yaml down -v
```
