# Messaging Example (NATS)

Demonstrates `wasi-messaging` backed by NATS JetStream for pub-sub and request-reply patterns.

## Prerequisites

Start NATS JetStream:

```bash
docker compose -f examples/messaging-nats/nats.yaml up -d
```

## Quick Start

```bash
./scripts/run-example.sh messaging-nats
```

## Test

```bash
# Pub-Sub pattern
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080/pub-sub

# Request-Reply pattern
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080/request-reply
```

## What It Does

**Pub-Sub:** Subscribes to topic `a.v1` and republishes messages to topic `b.v1`. HTTP requests initiate publishing to `a.v1` and generate 100 messages to `b.v1`.

**Request-Reply:** Sends a message to `req.v1` and waits for a reply on `rep.v1`. A subscriber listens on `req.v1` and replies to each message.

## Cleanup

```bash
docker compose -f examples/messaging-nats/nats.yaml down -v
```
