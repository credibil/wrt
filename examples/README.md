# Examples

Example runtimes can be used as a starting point for building and deploying WASI-based applications. The examples run in Docker containers but can readily be built and deployed as standalone binaries.

## Quick Start

To get started, add a `.env` file to the [runtimes](./runtimes) directory (see `.env.example`) and run:

```bash
docker compose --file ./examples/runtimes/compose.yaml up
```

This will start a wasm runtime running a simple HTTP server instrumented with logging and metrics.

## Docker Build

```bash
export CARGO_REGISTRIES_CREDIBIL_TOKEN="<registry token>"

docker build \
  --file ./examples/runtimes/Dockerfile \
  --build-arg PACKAGE=everything \
  --secret id=credibil,env=CARGO_REGISTRIES_CREDIBIL_TOKEN \
  --tag ghcr.io/credibil/wrt .
```
