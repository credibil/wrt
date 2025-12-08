# Messaging Example

Demonstrates `wasi-messaging` using the default (in-memory) implementation for pub-sub messaging.

## Quick Start

```bash
./scripts/run-example.sh messaging
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## What It Does

This example demonstrates basic pub-sub messaging patterns using an in-memory message broker.
