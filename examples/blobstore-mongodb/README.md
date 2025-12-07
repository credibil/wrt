# Blobstore Example (MongoDB)

Demonstrates `wasi-blobstore` backed by MongoDB for persistent blob storage.

## Prerequisites

Start MongoDB:

```bash
docker compose -f examples/blobstore-mongodb/mongodb.yaml up -d
```

## Quick Start

```bash
./scripts/run-example.sh blobstore-mongodb
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Cleanup

```bash
docker compose -f examples/blobstore-mongodb/mongodb.yaml down -v
```
