# Blobstore Example (NATS)

Demonstrates `wasi-blobstore` backed by NATS JetStream for persistent blob storage.

## Prerequisites

Start NATS JetStream:

```bash
docker compose -f examples/blobstore-nats/nats.yaml up -d
```

## Quick Start

```bash
# build the guest
cargo build --example blobstore-nats-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example blobstore-nats -- run ./target/wasm32-wasip2/debug/examples/blobstore_nats_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Cleanup

```bash
docker compose -f examples/blobstore-nats/nats.yaml down -v
```
