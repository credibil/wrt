# OTel Example

## Quick Start

To get started add a `.env` file to this folder. See the `.env.example` file in the workspace root for a template.

In a console, build and run the `otel` example:

```bash
# build the guest.
cargo build --example otel --target wasm32-wasip2 --release

Run the services in Docker containers using docker compose.

```bash
docker compose --file ./examples/otel/compose.yaml up
```

In a separate console, call the guest:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
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