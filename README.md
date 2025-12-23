# Credibil Wasm Runtime

The Credibil Wasm runtime provides a thin wrapper around [`wasmtime`](https://github.com/bytecodealliance/wasmtime)
for ergonomic integration of host-based services for WASI components.

We consider this a stop-gap solution until production-grade runtimes support dynamic inclusion of
host-based services.

## Examples

There are a number of examples provided in the `examples` directory that can be used to experiment
with the runtime and see it in action.

Each example contains a Wasm guest and the runtime required to run it.

See [examples/README.md](./examples/README.md) for more details.

## Building

There are multiple ways to build a runtime by combining `--bin` and `--features` flags.
For example, to build the `realtime` runtime with all features enabled:

```bash
cargo build --bin=realtime --features=realtime --release
```

### Docker

Building with Docker:

```bash
export CARGO_REGISTRIES_CREDIBIL_TOKEN="<registry token>"

docker build \
  --build-arg BIN="realtime" \
  --build-arg FEATURES="realtime" \
  --secret id=credibil,env=CARGO_REGISTRIES_CREDIBIL_TOKEN \
  --tag ghcr.io/credibil/realtime .
```