# Key-Value Example (Redis)

Demonstrates `wasi-keyvalue` backed by Redis for persistent key-value storage.

## Prerequisites

Start Redis:

```bash
docker compose -f examples/keyvalue-redis/redis.yaml up -d
```

## Quick Start

```bash
./scripts/run-example.sh keyvalue-redis
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Cleanup

```bash
docker compose -f examples/keyvalue-redis/redis.yaml down -v
```
