# Messaging Example

Demonstrates `wasi-messaging` using the default (in-memory) implementation for pub-sub messaging.

## Quick Start

```bash
# build the guest
cargo build --example messaging-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example messaging -- run ./target/wasm32-wasip2/debug/examples/messaging_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080/pub-sub
```
