# Identity Example

This example uses the default implementation of `wasi-identity`.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example identity --target wasm32-wasip2
```

### Run

Start the Otel Collector in a separate console:

```bash
docker compose --file ./docker/otelcol.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --bin identity-runtime --features http,identity,otel -- run ./target/wasm32-wasip2/debug/examples/identity.wasm
```

### Test

```bash
curl http://localhost:8080
```
