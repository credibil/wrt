# HTTP Server Example

Demonstrates a basic HTTP server using `wasi-http` with GET and POST endpoints.

## Quick Start

```bash
# build the guest
cargo build --example http-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example http -- run ./target/wasm32-wasip2/debug/examples/http_wasm.wasm
```

## Test

```bash
# POST request
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080

# GET request
curl http://localhost:8080
```

## What It Does

This example creates a simple HTTP server with GET and POST endpoints that echo back JSON responses.
