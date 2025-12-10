# Key-Value Example (Redis)

Demonstrates `wasi-keyvalue` backed by Redis for persistent key-value storage.

## Prerequisites

Start Redis:

```bash
docker compose -f examples/keyvalue-redis/redis.yaml up -d
```

## Quick Start

```bash
# build the guest
cargo build --example keyvalue-redis-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example keyvalue-redis -- run ./target/wasm32-wasip2/debug/examples/keyvalue_redis_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Cleanup

```bash
docker compose -f examples/keyvalue-redis/redis.yaml down -v
```
