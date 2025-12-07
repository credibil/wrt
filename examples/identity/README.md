# Identity Example

This example uses the default implementation of `wasi-identity`.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the guest

```bash
cargo build --example identity-wasm --target wasm32-wasip2
```

### Run

Start the Otel Collector in a separate console:

```bash
docker compose --file ./docker/otelcol.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --example identity -- run ./target/wasm32-wasip2/debug/examples/identity_wasm.wasm
```

### Test

```bash
curl http://localhost:8080
```
