## 0.12.0

Unreleased

### Added

* Support for HTTP proxy response caching
* Support for client auth certs in HTTP requests
* Redis, Kafka, and Postgres resources 

### Changed

Refactored WASI components to to include both guest and host in same crate. This allows for
both sides to be built from the same WIT definition, reducing duplication and errors.

Refactored runtime (and WASI components) to use customised run state. This allows for resources
to be injected into the component in a top-down manner and more in line with Wasmtime's approach.

Additionally:

* Updated to Wasmtime 38
* Updated all WASI components to wasi-p3
* Many bug fixes

**Full Changelog**: https://github.com/credibil/wrt/compare/v0.4.0...v0.12.0

---

Release notes for previous releases can be found on the respective release 
branches of the repository.

<!-- ARCHIVE_START -->
* [0.11.x](https://github.com/credibil/wrt/blob/release-0.11.0/RELEASES.md)
* [0.10.x](https://github.com/credibil/wrt/blob/release-0.10.0/RELEASES.md)
* [0.9.x](https://github.com/credibil/wrt/blob/release-0.9.0/RELEASES.md)
* [0.8.x](https://github.com/credibil/wrt/blob/release-0.8.0/RELEASES.md)
* [0.7.x](https://github.com/credibil/wrt/blob/release-0.7.0/RELEASES.md)
* [0.6.x](https://github.com/credibil/wrt/blob/release-0.6.0/RELEASES.md)
* [0.5.x](https://github.com/credibil/wrt/blob/release-0.5.0/RELEASES.md)
* [0.4.x](https://github.com/credibil/wrt/blob/release-0.4.0/RELEASES.md)
* [0.3.x](https://github.com/credibil/wrt/blob/release-0.3.0/RELEASES.md)
* [0.2.x](https://github.com/credibil/wrt/blob/release-0.2.0/RELEASES.md)
* [0.1.x](https://github.com/credibil/core/blob/release-0.1.0/RELEASES.md)
