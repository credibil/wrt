# Key-Value Example (NATS)

This example implements a simple key-value store using `wasi-keyvalue` backed by NATS JetStream.

## Quick Start

1. Optional: copy `.env.example` to the repo root as `.env`.
2. Build the guest:
   ```bash
   cargo build --example keyvalue-nats-wasm --target wasm32-wasip2
   ```
3. Start dependencies (in another terminal):
   ```bash
   docker compose --file ./examples/keyvalue-nats/nats.yaml up
   ```
4. Run the host + guest:
   ```bash
   bash scripts/env-run.sh cargo run --example keyvalue-nats -- run ./target/wasm32-wasip2/debug/examples/keyvalue_nats_wasm.wasm
   ```
5. Test:
   ```bash
   curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
   ```
