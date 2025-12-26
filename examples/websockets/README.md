# WebSockets Server Example

Demonstrates `wasi-websockets` for real-time bidirectional communication.

## Quick Start

```bash
# build the guest
cargo build --example websockets-wasm --target wasm32-wasip2

# run the host
export RUST_LOG="info,wasi_websockets=debug,wasi_http=debug,websockets=debug"
cargo run --example websockets -- run ./target/wasm32-wasip2/debug/examples/websockets_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

