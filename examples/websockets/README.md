# WebSockets Server Example

Demonstrates `wasi-websockets` for real-time bidirectional communication.

## Quick Start

```bash
./scripts/run-example.sh websockets
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## What It Does

This example creates a WebSocket server that can handle real-time, bidirectional communication with clients.
