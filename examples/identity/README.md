# Identity Example

Demonstrates `wasi-identity` using the default implementation.

## Quick Start

```bash
# build the guest
cargo build --example identity-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example identity -- run ./target/wasm32-wasip2/debug/examples/identity_wasm.wasm
```

## Test

```bash
curl http://localhost:8080
```

## What It Does

This example demonstrates identity/authentication capabilities within a WASI guest module.
