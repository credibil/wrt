# Key Value Example

Build the example guest:

```bash
cargo build --package wasm-keyvalue --target wasm32-wasip2 --release
```

Run the example guest:

```bash
# compile and run
cargo run --package everything -- run ./target/wasm32-wasip2/release/wasm_keyvalue.wasm

# pre-compile
cargo run -- compile  ./target/wasm32-wasip2/release/wasm_keyvalue.wasm --output ./wasm_keyvalue.bin
cargo run -- run ./wasm_keyvalue.bin
```

In a separate console, send a messages to the guest:

```bash
curl -d '{"text":"hello"}' http://localhost:8080
```
