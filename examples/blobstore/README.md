# Blobstore Example

This example implements a simple blobstore using `wasi-blobstore` backed by either NATS JetStream
or Azure Blob Storage.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a template.

#### Build

```bash
cargo build --example blobstore --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a

# NATS
cargo run --features http,otel,blobstore,nats -- run ./target/wasm32-wasip2/release/examples/blobstore.wasm

# Redis
cargo run --features http,otel,blobstore,redis -- run ./target/wasm32-wasip2/release/examples/blobstore.wasm
```

Alternatively, using Docker Compose:

```bash
# NATS
docker compose --file ./examples/blobstore/nats.yaml up

# Azure Blob Storage
docker compose --file ./examples/blobstore/azurebs.yaml up
```

#### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```