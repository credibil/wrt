# Blobstore Example

This example implements a simple blobstore using `wasi-blobstore` backed by either NATS JetStream
or MongoDB.

## Quick Start

This example uses the default implementation of `wasi-blobstore`.

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

```bash
# build the guest
cargo build --example blobstore --all-features --target wasm32-wasip2

# console 1: run the guest
set -a && source .env && set +a
cargo run --all-features -- run ./target/wasm32-wasip2/debug/examples/blobstore.wasm

# console 2: test the guest
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

### NATS

```bash
# console 1
docker compose --file ./examples/blobstore/nats.yaml up

# console 2
set -a && source .env && set +a
cargo run --bin blobstore-nats --features http,otel,blobstore,nats -- run ./target/wasm32-wasip2/debug/examples/blobstore.wasm
```

### MongoDB

```bash
# console 1
docker compose --file ./examples/blobstore/mongodb.yaml up

# console 2
set -a && source .env && set +a
cargo run --bin blobstore-mongodb --features http,otel,blobstore,mongodb -- run ./target/wasm32-wasip2/debug/examples/blobstore.wasm
```
