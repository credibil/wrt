# Blobstore NATS Example

This example implements a simple blobstore using `wasi-blobstore` backed by NATS JetStream.

## Quick Start

This example uses NATS as a backend to `wasi-blobstore`.

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

```bash
# build the guest
cargo build --example blobstore-nats --all-features --target wasm32-wasip2

# console 1: start NATS
docker compose --file ./examples/blobstore-nats/nats.yaml up

# console 2: run the guest
set -a && source .env && set +a
cargo run --bin blobstore-nats -- run ./target/wasm32-wasip2/debug/blobstore_nats.wasm
```
