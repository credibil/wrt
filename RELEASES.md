## 0.13.0

Unreleased

### Added

Realtime WASI components and resources have been moved here temporarily while
we work on a dedicated Realtime runtime.

Several new WASI components have been added:
- wasi-sql
- wasi-identity
- wasi-websockets

In addition, new component resources have been added:
- Postgres
- Redis
- Kafka

### Changed

The Docker image release process has been optimised by using a GitHub Actions 
that matches the target architecture of the image. Builds now complete in under
15 minutes.

`wasmtime` has been updated to version 38 allowing us to take advantage of 
the new support for `wasip3`. All WASI components have been migrated to 
support async and concurrency.

---

Release notes for previous releases can be found on the respective release 
branches of the repository.

<!-- ARCHIVE_START -->
* [0.12.x](https://github.com/credibil/wrt/blob/release-0.12.0/RELEASES.md)
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
