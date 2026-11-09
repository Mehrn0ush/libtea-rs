# libtea-core

Independent Rust types and protocol rules for the [CycloneDX Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api). This crate does not open network connections.

**Independent implementation.** Not an official CycloneDX project. No conformance
certification. [`IntegrityOutcome::Verified`](https://docs.rs/libtea-core/latest/libtea_core/enum.IntegrityOutcome.html)
means declared digests matched decoded bytes only — **not** publisher authenticity.
No signature verification lives in this crate.

Implemented against Transparency Exchange API commit
`962a0228af1bfcc1232ac3f79239317ff33fdb04`. The TEA specification is still beta.

```toml
[dependencies]
libtea-core = "0.1.0"
```

The snippet below needs nothing else. For the async client add `libtea-client` (its README lists the dependencies its example needs).

Edition 2021. MSRV 1.89. License Apache-2.0.

**MSRV.** The minimum supported Rust version is 1.89 for 0.1.0. Policy: the latest stable release minus about four releases; it is raised only in minor releases, and every raise is listed in the `CHANGELOG.md`. Dependencies use plain caret ranges and `Cargo.lock` is committed, so MSRV is verified with the committed lock file (`cargo +1.89.0 test --workspace --all-targets --locked`).

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
