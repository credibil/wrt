# Websockets Server Example

This example implements a simple websockets server using `wasi-websockets`. 

## Quick Start

To get started add a `.env` file to the workspace root. See `.env.example` for a template.

#### Build

```bash
cargo build --example websockets --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a
cargo run --features http,otel,websockets -- run ./target/wasm32-wasip2/release/examples/websockets.wasm
```

Docker Compose can also be used to run the service:

```bash
docker compose --file ./examples/websockets/websockets.yaml up
```

#### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
