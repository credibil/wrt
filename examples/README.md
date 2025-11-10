# Examples

Example runtimes can be used as a starting point for building and deploying WASI-based 
applications. The examples run in Docker containers but can readily be built and deployed
as standalone binaries.

## Quick Start

To get started add a `.env` file to the root of the project (see `.env.example`).

In a console, build and run the `http` example (or any other example):

```bash
# build the guest
cargo build --example http --target wasm32-wasip2 --release

# run the guest
set -a && source .env && set +a
cargo run --features "realtime-http" -- run ./target/wasm32-wasip2/release/examples/http.wasm
```

In a separate console, call the guest:

```bash
curl -H "Content-Type: application/json" -d '{"text":"hello"}' http://localhost:8080
```

## OpenTelemetry Collector

```bash
docker compose --file ./examples/opentelemetry/compose.yaml up
```

This will start a wasm runtime running a simple HTTP server instrumented with logging and metrics.

## Docker Build

```bash
export CARGO_REGISTRIES_CREDIBIL_TOKEN="<registry token>"

docker build \
  --build-arg FEATURES=http-server \
  --secret id=credibil,env=CARGO_REGISTRIES_CREDIBIL_TOKEN \
  --tag ghcr.io/credibil/wrt .
```