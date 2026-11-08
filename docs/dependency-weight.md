# libtea-client dependency weight (local measure 2026-11-06T11:06Z)

## cargo tree (libtea-client, depth 1)
libtea-client v0.1.0 (crates/tea-client)
├── base64 v0.22.1
├── bytes v1.12.1
├── flate2 v1.1.10
├── futures-util v0.3.34
├── libtea-core v0.1.0 (crates/tea-core)
├── reqwest v0.12.28
├── serde v1.0.229
├── serde_json v1.0.151
├── thiserror v2.0.21
├── tokio v1.54.1
├── tracing v0.1.44
└── url v2.5.4

## Unique crates in tree (approx)
     162

## Build timing (release, quiet)
real 73.62
user 172.27
sys 22.01

## Example binary size (`paginate_components`, release)
- On-disk: ~5.4 MiB (`5379792` bytes) for a release example linking the client.
- Not a minimal CLI strip; indicative of client + rustls footprint only.

## Notes
- TLS backend today: reqwest + rustls only.
- futures-util already required by reqwest stream feature; follow_pages adds no new dep.
- Proposed optional features documented in README; not implemented for 0.1.0 defaults.

## F1 streaming download (no new crate)

- `tokio` gains the `fs` feature (`tokio::fs::File` for the temporary file). It
  pulls in no new package: `Cargo.lock` is unchanged by it. `io-util` was
  already enabled.
- `libtea-core`, `libtea-client` and `libtea-cli` add no dependency. `flate2`
  and `sha2` were already dependencies; the tests use them as before.

## F3 release workflow (no new crate dependency)

- `cargo-cyclonedx` is installed in the release workflow's `sbom` job only
  (`cargo install --locked --version`, version pinned in `release.yml`). It is
  not a dependency of any workspace crate and does not touch `Cargo.lock`.
- The scripts use Python 3 (standard library only) and Bash.

## F4 GitHub Action (no new crate dependency)

- The action adds no Rust dependency. Its scripts use Bash, `curl`, and the
  Python 3 standard library (`tarfile`, `zipfile`); the one third-party action is
  `actions/cache` (placeholder pin).

## F6 cache (no new crate dependency)

- `libtea-core` already depended on `sha2`; the cache entry format and keys use it.
  `libtea-client` adds no dependency (file work uses `std::fs` and the existing
  `tokio` `fs` feature). `Cargo.lock` is unchanged.
