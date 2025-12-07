# Websockets Server Example

This example implements a simple websockets server using `wasi-websockets`.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example websockets-wasm --target wasm32-wasip2
```

Run the host + guest:

```bash
bash scripts/env-run.sh cargo run --example websockets -- run ./target/wasm32-wasip2/debug/examples/websockets_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
