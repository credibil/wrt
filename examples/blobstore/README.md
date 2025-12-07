# Blobstore Example

This example implements a simple blobstore using `wasi-blobstore`.

## Quick Start

This example uses the default implementation of `wasi-blobstore`.

1. Optional: copy `.env.example` to the repo root as `.env`.
2. Build the guest:
   ```bash
   cargo build --example blobstore-wasm --target wasm32-wasip2
   ```
3. Run the host + guest:
   ```bash
   bash scripts/env-run.sh cargo run --example blobstore -- run ./target/wasm32-wasip2/debug/examples/blobstore_wasm.wasm
   ```
4. Test:
   ```bash
   curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
   ```
