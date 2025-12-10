# Blobstore Example (MongoDB)

Demonstrates `wasi-blobstore` backed by MongoDB for persistent blob storage.

## Prerequisites

Start MongoDB:

```bash
docker compose -f examples/blobstore-mongodb/mongodb.yaml up -d
```

## Quick Start

```bash
# build the guest
cargo build --example blobstore-mongodb-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example blobstore-mongodb -- run ./target/wasm32-wasip2/debug/examples/blobstore_mongodb_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Cleanup

```bash
docker compose -f examples/blobstore-mongodb/mongodb.yaml down -v
```
