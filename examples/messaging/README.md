# Messaging Example

This example implements `wasi-messaging` using the default (in memory) implementation.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example messaging-wasm --target wasm32-wasip2
```

Run the host + guest:

```bash
bash scripts/env-run.sh cargo run --example messaging -- run ./target/wasm32-wasip2/debug/examples/messaging_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
