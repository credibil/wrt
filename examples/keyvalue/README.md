# Key-Value Example

Demonstrates `wasi-keyvalue` using the default (in-memory) implementation.

## Quick Start

```bash
# build the guest
cargo build --example keyvalue-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example keyvalue -- run ./target/wasm32-wasip2/debug/examples/keyvalue_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## What It Does

This example creates an HTTP endpoint that:

- Accepts data via POST
- Stores it in an in-memory key-value store
- Retrieves it to verify the operation
- Returns a success response
