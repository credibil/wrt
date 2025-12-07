# Key-Value Example (NATS)

Demonstrates `wasi-keyvalue` backed by NATS JetStream for persistent key-value storage.

## Prerequisites

Start NATS JetStream:

```bash
docker compose -f examples/keyvalue-nats/nats.yaml up -d
```

## Quick Start

```bash
./scripts/run-example.sh keyvalue-nats
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Cleanup

```bash
docker compose -f examples/keyvalue-nats/nats.yaml down -v
```
