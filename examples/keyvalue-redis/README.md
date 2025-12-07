# Key-Value Example (Redis)

This example implements a simple key-value store using `wasi-keyvalue` backed by Redis.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example keyvalue-redis-wasm --target wasm32-wasip2
```

Start dependencies (in another terminal):

```bash
docker compose --file ./examples/keyvalue-redis/redis.yaml up
```

Run the host + guest:

```bash
bash scripts/env.sh cargo run --example keyvalue-redis -- run ./target/wasm32-wasip2/debug/examples/keyvalue_redis_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
