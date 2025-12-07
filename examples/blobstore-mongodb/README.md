# Blobstore MongoDB Example

This example implements a simple blobstore using `wasi-blobstore` backed by MongoDB.

## Quick Start

This example uses MongoDB as a backend to `wasi-blobstore`.

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

```bash
# build the guest
cargo build --example blobstore-mongodb-wasm --target wasm32-wasip2

# console 1: start MongoDB
docker compose --file ./examples/blobstore-mongodb/mongodb.yaml up

# console 2: run the guest
set -a && source .env && set +a
cargo run --example blobstore-mongodb -- run ./target/wasm32-wasip2/debug/examples/blobstore_mongodb_wasm.wasm
```
