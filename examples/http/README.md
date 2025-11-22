# HTTP Server Example

This example implements a simple HTTP server using `wasi-http`. 

## Quick Start

To get started add a `.env` file to the workspace root. See `.env.example` for a template.

#### Build

```bash
cargo build --example http --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a
cargo run --features http-server -- run ./target/wasm32-wasip2/release/examples/http.wasm
```

Docker Compose can also be used to run the service:

```bash
docker compose --file ./examples/http/hyper.yaml up
```

#### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
