# Blobstore Example

This example implements `wasi-blobstore` using the default (in memory) implementation.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example blobstore-wasm --target wasm32-wasip2
```

Run the host + guest:

```bash
bash scripts/env.sh cargo run --example blobstore -- run ./target/wasm32-wasip2/debug/examples/blobstore_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
