# SQL Example (Postgres)

Demonstrates `wasi-sql` backed by PostgreSQL for persistent database operations.

## Prerequisites

Start PostgreSQL:

```bash
docker compose -f examples/sql-postgres/postgres.yaml up -d
```

## Quick Start

```bash
./scripts/run-example.sh sql-postgres
```

## Test

```bash
# 1. INSERT a row
curl -X POST --header 'Content-Type: application/json' -d '{}' http://localhost:8080/

# 2. SELECT all rows
curl http://localhost:8080/
```

## What It Does

- HTTP POST: Inserts a row into the database
- HTTP GET: Queries and returns all rows from the sample table

## Cleanup

```bash
docker compose -f examples/sql-postgres/postgres.yaml down -v
```
