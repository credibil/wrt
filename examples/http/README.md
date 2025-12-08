# HTTP Server Example

Demonstrates a basic HTTP server using `wasi-http` with GET and POST endpoints.

## Quick Start

```bash
./scripts/run-example.sh http
```

## Test

```bash
# POST request
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080

# GET request
curl http://localhost:8080
```

## What It Does

This example creates a simple HTTP server with GET and POST endpoints that echo back JSON responses.
