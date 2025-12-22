## 0.19.0

Unreleased

### Added

**Runtime Build Generation (`buildgen`)** - The `runtime!` macro now generates the necessary runtime infrastructure for executing WebAssembly components with WASI capabilities, replacing the previous feature flag approach. This is a significant architectural improvement.

**Runtime Configuration (`wasi-config`)** - New `WasiConfig` struct allows you to configure the runtime for specific components.

**Centralized Guest Capabilities (`fabric`)** - New crate to share common capabilities (traits and error handling) between guest components.

**Default Implementations** - All `wasi-xxx` components now have default implementations to streamline examples and improve developer experience.

**WASI Identity Support** - Client credentials implementation with `wasi-identity` default context.

### Changed

- **Wasmtime 40.0.0 Upgrade** - Updated to latest wasmtime version with stricter linking requirements
- **Async WITs** - Selectively added `async` to existing WIT methods, allowing removal of blanket async implementation in `wasmtime::component::bindgen`
- **Parallel WASM Compilation** - Enabled parallel compilation for improved performance
- **Error Handling** - Centralized error handling across the project
- **Serialization** - Replaced `bincode` with `rkyv` for serialization
- **Messaging** - Switched to using Tokio channels for in-memory messaging
- **Backend Naming** - Renamed backends from `res-xxx` to `be-xxx`
- **Runtime Crate** - Renamed `runtime` crate to `kernel`
- **Examples** - Streamlined examples by adding a `runtime.rs` file to each example directory

### Details

This release represents a significant architectural evolution with **15 commits**, **255 files changed**, and contributions from **5 contributors**. The focus is on improving developer experience, code organization, and runtime flexibility

---

Release notes for previous releases can be found on the respective release
branches of the repository.

<!-- ARCHIVE_START -->
* [0.18.x](https://github.com/credibil/wrt/blob/release-0.18.0/RELEASES.md)
* [0.17.x](https://github.com/credibil/wrt/blob/release-0.17.0/RELEASES.md)
* [0.16.x](https://github.com/credibil/wrt/blob/release-0.16.0/RELEASES.md)
* [0.15.x](https://github.com/credibil/wrt/blob/release-0.15.0/RELEASES.md)
