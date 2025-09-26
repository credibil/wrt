# Examples

Example runtimes can be used as a starting point for building and deploying WASI-based applications. The examples run in Docker containers but can readily be built and deployed as standalone binaries.

## Quick Start

To get started, add a `.env` file to the root of the project (see `.env.example`) and run:

```bash
docker compose up
```

This will start a wasm runtime running a simple HTTP server instrumented with logging and metrics.

## Docker Build

```bash
export CARGO_REGISTRIES_CREDIBIL_TOKEN="<registry token>"

docker build \
  --build-arg BIN=standard \
  --secret id=credibil,env=CARGO_REGISTRIES_CREDIBIL_TOKEN \
  --tag ghcr.io/credibil/wrt .

#  OR 

docker build \
  --build-arg BIN=minimal \
  --secret id=credibil,env=CARGO_REGISTRIES_CREDIBIL_TOKEN \
  --tag ghcr.io/credibil/wrt .
```