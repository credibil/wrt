# Blobstore Example

This example implements a simple blobstore using `wasi-blobstore` backed by either NATS JetStream
or MongoDB.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example blobstore --target wasm32-wasip2 --release
```

### Run with Cargo

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./examples/docker/opentelemetry.yaml up
```

Run the guest using either NATS or MongoDB:

```bash
# load env vars
set -a && source .env && set +a

# with NATS
cargo run --features http,otel,blobstore,nats -- run ./target/wasm32-wasip2/release/examples/blobstore.wasm

# with MongoDB
cargo run --features http,otel,blobstore,mongodb -- run ./target/wasm32-wasip2/release/examples/blobstore.wasm
```

### Run with Docker Compose

Docker Compose provides an easy way to run the example with all dependencies.

```bash
# with NATS
docker compose --file ./examples/blobstore/nats.yaml up

# with MongoDB
docker compose --file ./examples/blobstore/mongodb.yaml up
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```