# libtea-core

Independent Rust types and protocol rules for the [CycloneDX Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api). This crate does not open network connections.

**Independent implementation.** Not an official CycloneDX project. No conformance
certification. [`IntegrityOutcome::Verified`](https://docs.rs/libtea-core/latest/libtea_core/enum.IntegrityOutcome.html)
means declared digests matched decoded bytes only — **not** publisher authenticity.
No signature verification lives in this crate.

Implemented against Transparency Exchange API commit
`962a0228af1bfcc1232ac3f79239317ff33fdb04`.

```toml
libtea-core = "0.1.0"
```

Edition 2021. MSRV 1.85. License Apache-2.0.

**MSRV and dependency resolution.** MSRV 1.85 is verified with this repository's committed `Cargo.lock`, resolved by the MSRV-aware resolver (`resolver = "3"`). The dependencies use caret ranges, so cargo only keeps to 1.85 when it resolves with MSRV awareness: consumers on older toolchains should use `resolver = "3"` (the default for edition 2024), set `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback`, or commit a lock file. Without that, cargo can select newer releases that need a newer Rust (for example `uuid` 1.29 needs 1.89, `idna_adapter` 1.2.2 needs 1.86, and `icu_*` 2.4 needs 1.88).

This snippet is compiled as a doctest. Another example that builds is `examples/parse_tei.rs`.

```rust
use libtea_core::Tei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1")?;
    assert_eq!(tei.tei_type(), "uuid");
    // The legacy `urn:tei:` form is rejected with its own error.
    assert!(Tei::parse("urn:tei:uuid:example.com:d4d9f54a-abcf-11ee-ac79-1a52914d44b1").is_err());
    Ok(())
}
```

**Semver surface.** `url::Url`, `semver::Version` and `uuid::Uuid` appear in public signatures (`DestinationPolicy::check_url`, `ApiVersion::semver`, `TeaUuid::as_uuid`). They are re-exported as `libtea_core::{url, semver, uuid}`, so use those paths to get exactly the version this crate was built against. A major-version bump of any of the three is a breaking change of `libtea-core`.

HTTP is in [`libtea-client`](https://crates.io/crates/libtea-client). The workspace guide is at <https://github.com/Mehrn0ush/libtea-rs>.
