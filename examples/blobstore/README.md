# Blobstore Example

Demonstrates `wasi-blobstore` using the default (in-memory) implementation.

## Quick Start

```bash
# build the guest
cargo build --example blobstore-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example blobstore -- run ./target/wasm32-wasip2/debug/examples/blobstore_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## What It Does

This example creates an HTTP endpoint that:

- Accepts JSON data via POST
- Writes the data to an in-memory blobstore container
- Reads it back to verify the operation
- Returns the data as JSON
