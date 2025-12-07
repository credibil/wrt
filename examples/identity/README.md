# Identity Example

This example uses the default implementation of `wasi-identity`.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example identity-wasm --target wasm32-wasip2
```

Start dependencies (in another terminal):

```bash
docker compose --file ./docker/otelcol.yaml up
```

Run the host + guest:

```bash
bash scripts/env-run.sh cargo run --example identity -- run ./target/wasm32-wasip2/debug/examples/identity_wasm.wasm
```

Test:

```bash
curl http://localhost:8080
```
