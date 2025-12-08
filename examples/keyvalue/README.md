# Key-Value Example

Demonstrates `wasi-keyvalue` using the default (in-memory) implementation.

## Quick Start

```bash
./scripts/run-example.sh keyvalue
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## What It Does

This example creates an HTTP endpoint that:
- Accepts data via POST
- Stores it in an in-memory key-value store
- Retrieves it to verify the operation
- Returns a success response
