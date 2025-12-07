# Blobstore NATS Example

This example implements a simple blobstore using `wasi-blobstore` backed by NATS JetStream.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example blobstore-nats-wasm --target wasm32-wasip2
```

Start dependencies (in another terminal):

```bash
docker compose --file ./examples/blobstore-nats/nats.yaml up
```

Run the host + guest:

```bash
bash scripts/env-run.sh cargo run --example blobstore-nats -- run ./target/wasm32-wasip2/debug/examples/blobstore_nats_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
