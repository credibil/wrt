# Blobstore MongoDB Example

This example implements a simple blobstore using `wasi-blobstore` backed by MongoDB.

## Quick Start

This example uses MongoDB as a backend to `wasi-blobstore`.

1. Optional: copy `.env.example` to the repo root as `.env`.
2. Build the guest:
   ```bash
   cargo build --example blobstore-mongodb-wasm --target wasm32-wasip2
   ```
3. Start dependencies (in another terminal):
   ```bash
   docker compose --file ./examples/blobstore-mongodb/mongodb.yaml up
   ```
4. Run the host + guest:
   ```bash
   bash scripts/env-run.sh cargo run --example blobstore-mongodb -- run ./target/wasm32-wasip2/debug/examples/blobstore_mongodb_wasm.wasm
   ```
