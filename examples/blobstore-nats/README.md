# Blobstore Example (NATS)

Demonstrates `wasi-blobstore` backed by NATS JetStream for persistent blob storage.

## Prerequisites

Start NATS JetStream:

```bash
docker compose -f examples/blobstore-nats/nats.yaml up -d
```

## Quick Start

```bash
./scripts/run-example.sh blobstore-nats
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Cleanup

```bash
docker compose -f examples/blobstore-nats/nats.yaml down -v
```
