# Attribution for vendored specification files

The files under `spec/vendor/` are unmodified copies of files from the
OWASP / CycloneDX Transparency Exchange API repository:

- Upstream: https://github.com/CycloneDX/transparency-exchange-api
- Commit: `962a0228af1bfcc1232ac3f79239317ff33fdb04` (2026-10-08T06:28:26Z)
- Per-file upstream paths and SHA-256 digests: `spec/PINNED.md`,
  `spec/manifest.json`

License: the vendored `spec/vendor/openapi.yaml` declares
`info.license.identifier: Apache-2.0` and points to the upstream `LICENSE` file.
The Apache License 2.0 text is in this repository's root `LICENSE`. The
upstream `LICENSE` and any upstream `NOTICE` file are not vendored. Re-check
them at the pinned commit when re-pinning (`spec/RE-PINNING.md`).

These files are specification inputs for tests and documentation. They are not
packaged into the published `libtea-core` or `libtea-client` crates (check
with `cargo package --list`). Vendoring them does not mean the upstream
project endorses this implementation.
