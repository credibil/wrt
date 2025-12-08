# Vault Example

Demonstrates `wasi-vault` using the default (in-memory) implementation for secure secret storage.

## Quick Start

```bash
./scripts/run-example.sh vault
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## What It Does

This example demonstrates basic secret management capabilities using an in-memory vault implementation.
