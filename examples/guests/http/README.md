# Http Example

Build the example guest:

```bash
cargo build --package wasm-http --target wasm32-wasip2 --release
```

Run the example guest:

```bash
# compile and run
cargo run --package minimal -- ./target/wasm32-wasip2/release/wasm_http.wasm

# pre-compile
cargo run -- compile  ./target/wasm32-wasip2/release/wasm_http.wasm --output ./wasm_http.bin
cargo run -- run ./wasm_http.bin
```

In a separate console, call the guest:

```bash
curl -d '{"text":"hello"}' http://localhost:8080
```