# Blobstore NATS Example

This example implements a simple blobstore using `wasi-blobstore` backed by NATS JetStream.

## Quick Start

This example uses NATS as a backend to `wasi-blobstore`.

1. Optional: copy `.env.example` to the repo root as `.env`.
2. Build the guest:
   ```bash
   cargo build --example blobstore-nats-wasm --target wasm32-wasip2
   ```
3. Start dependencies (in another terminal):
   ```bash
   docker compose --file ./examples/blobstore-nats/nats.yaml up
   ```
4. Run the host + guest:
   ```bash
   bash scripts/env-run.sh cargo run --example blobstore-nats -- run ./target/wasm32-wasip2/debug/examples/blobstore_nats_wasm.wasm
   ```
