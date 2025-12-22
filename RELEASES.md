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

<!-- Release notes generated using configuration in .github/release.yaml at main -->

## What's Changed
* Bump to 0.18.0 by @github-actions[bot] in https://github.com/credibil/wrt/pull/114
* Release issues by @andrewweston in https://github.com/credibil/wrt/pull/117
* Delete .DS_Store by @andrewweston in https://github.com/credibil/wrt/pull/118
* Delete tmp.txt by @andrewweston in https://github.com/credibil/wrt/pull/119
* add container_name to shared compose files by @andrewweston in https://github.com/credibil/wrt/pull/120
* rename opentelemetry compose by @andrewweston in https://github.com/credibil/wrt/pull/121
* Rewrite OpenTelemetry export from HTTP to gRPC by @Copilot in https://github.com/credibil/wrt/pull/122
* Fix for the "cached response headers serialization" issue by @karthik-phl in https://github.com/credibil/wrt/pull/123
* Simplify Examples by @andrewweston in https://github.com/credibil/wrt/pull/124
* Bump to 0.19.0 by @github-actions[bot] in https://github.com/credibil/wrt/pull/125
* Use git ref for wasmtime by @andrewweston in https://github.com/credibil/wrt/pull/127
* Client credentials by @andrewweston in https://github.com/credibil/wrt/pull/128
* Enable parallel wasm compilation by @moritzdrexl-PHL in https://github.com/credibil/wrt/pull/116
* Runtime Build Generator by @andrewweston in https://github.com/credibil/wrt/pull/129
* Feature/centralise error handling by @quynhduongphl in https://github.com/credibil/wrt/pull/115
* Change log_with_metrics to only target non-wasm32 by @quynhduongphl in https://github.com/credibil/wrt/pull/130
* Refactor examples by @andrewweston in https://github.com/credibil/wrt/pull/131
* Comply with wasmtime's stricter linking requirements by @andrewweston in https://github.com/credibil/wrt/pull/134
* Providers by @andrewweston in https://github.com/credibil/wrt/pull/135
* Async WITs by @andrewweston in https://github.com/credibil/wrt/pull/136
* Capabilities by @andrewweston in https://github.com/credibil/wrt/pull/137
* Bincode by @andrewweston in https://github.com/credibil/wrt/pull/138
* Wasi Config by @andrewweston in https://github.com/credibil/wrt/pull/139

## New Contributors
* @Copilot made their first contribution in https://github.com/credibil/wrt/pull/122
* @karthik-phl made their first contribution in https://github.com/credibil/wrt/pull/123
* @moritzdrexl-PHL made their first contribution in https://github.com/credibil/wrt/pull/116

**Full Changelog**: https://github.com/credibil/wrt/compare/v0.17.0...v0.19.0

---

Release notes for previous releases can be found on the respective release
branches of the repository.

<!-- ARCHIVE_START -->
* [0.18.x](https://github.com/credibil/wrt/blob/release-0.18.0/RELEASES.md)
* [0.17.x](https://github.com/credibil/wrt/blob/release-0.17.0/RELEASES.md)
* [0.16.x](https://github.com/credibil/wrt/blob/release-0.16.0/RELEASES.md)
* [0.15.x](https://github.com/credibil/wrt/blob/release-0.15.0/RELEASES.md)
