# Key-Value Example (NATS)

This example implements a simple key-value store using `wasi-keyvalue` backed by NATS JetStream.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example keyvalue-nats-wasm --target wasm32-wasip2
```

Start dependencies (in another terminal):

```bash
docker compose --file ./examples/keyvalue-nats/nats.yaml up
```

Run the host + guest:

```bash
bash scripts/env.sh cargo run --example keyvalue-nats -- run ./target/wasm32-wasip2/debug/examples/keyvalue_nats_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
