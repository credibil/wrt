# Key-Value Example (Redis)

This example implements a simple key-value store using `wasi-keyvalue` backed by Redis.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example keyvalue-redis-wasm --target wasm32-wasip2
```

### Run Redis

Start the Redis server and Otel Collector in a separate console:

```bash
docker compose --file ./examples/keyvalue-redis/redis.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --example keyvalue-redis -- run ./target/wasm32-wasip2/debug/examples/keyvalue_redis_wasm.wasm
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
