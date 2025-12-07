# Identity Example

This example uses the default implementation of `wasi-identity`.

## Quick Start

1. Optional: copy `.env.example` to the repo root as `.env`.
2. Build the guest:
   ```bash
   cargo build --example identity-wasm --target wasm32-wasip2
   ```
3. Start dependencies (in another terminal):
   ```bash
   docker compose --file ./docker/otelcol.yaml up
   ```
4. Run the host + guest:
   ```bash
   bash scripts/env-run.sh cargo run --example identity -- run ./target/wasm32-wasip2/debug/examples/identity_wasm.wasm
   ```
5. Test:
   ```bash
   curl http://localhost:8080
   ```
