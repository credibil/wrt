# OpenTelemetry Example

Demonstrates OpenTelemetry instrumentation for WebAssembly guests using `wasi-otel`.

## Prerequisites

Start the OpenTelemetry Collector:

```bash
docker compose -f docker/otelcol.yaml up -d
```

## Quick Start

```bash
./scripts/run-example.sh otel
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## What It Does

This example demonstrates:
- Distributed tracing with OpenTelemetry
- Metrics collection from WebAssembly guests
- Integration with the OpenTelemetry Collector

## Cleanup

```bash
docker compose -f docker/otelcol.yaml down -v
```
