# HTTP Server Example

This example implements a simple HTTP server using `wasi-http`.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example http-wasm --target wasm32-wasip2
```

Run the host + guest:

```bash
bash scripts/env.sh cargo run --example http -- run ./target/wasm32-wasip2/debug/examples/http_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
