# Vault Example (Azure)

Demonstrates `wasi-vault` backed by Azure Key Vault for secure secret storage.

## Prerequisites

- Azure Key Vault instance
- Appropriate credentials configured in `.env`

## Quick Start

1. Copy `.env.example` to `.env` and configure Azure credentials
2. Run the example:

```bash
./scripts/run-example.sh vault-azure
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## What It Does

This example demonstrates secure secret management using Azure Key Vault as the backend for `wasi-vault`.
