# WebAssembly Interface Types (WIT) Deps

## Precursors

Install `wit-deps` from source (https://github.com/bytecodealliance/wit-deps)

## Usage

Add dependencies to `deps.toml`:

```toml
keyvalue = "https://github.com/credibil/wasi-keyvalue/archive/main.tar.gz"  
```

Import/update dependencies with:

```bash
wit-deps update
```