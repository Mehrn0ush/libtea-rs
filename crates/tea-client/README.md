# libtea-client

Independent async Rust client for the [CycloneDX Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api).

**Independent implementation.** Not an official CycloneDX project. No conformance
certification. No cryptographic signature verification. Checksum
`IntegrityOutcome::Verified` is **not** publisher authenticity.

Implemented against Transparency Exchange API commit
`962a0228af1bfcc1232ac3f79239317ff33fdb04`.

Publish `libtea-core` 0.1.0 before this crate. This crate depends on that version and does not depend on the unpublished CLI or conformance crates.

```toml
libtea-client = "0.1.0"
```

Edition 2021. MSRV 1.85. License Apache-2.0. TLS uses rustls. Certificate verification stays enabled. The default destination policy allows HTTPS to public addresses.

**MSRV and dependency resolution.** MSRV 1.85 is verified with this repository's committed `Cargo.lock`, resolved by the MSRV-aware resolver (`resolver = "3"`). The dependencies use caret ranges, so cargo only keeps to 1.85 when it resolves with MSRV awareness: consumers on older toolchains should use `resolver = "3"` (the default for edition 2024), set `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback`, or commit a lock file. Without that, cargo can select newer releases that need a newer Rust (for example `uuid` 1.29 needs 1.89, `idna_adapter` 1.2.2 needs 1.86, and `icu_*` 2.4 needs 1.88).

This snippet is compiled as a doctest but marked `no_run`, because it would contact a network host. Other examples that build are `examples/filter_by_identifier_type.rs` and `examples/paginate_components.rs`.

```rust,no_run
use libtea_client::Client;
use libtea_core::enums::IdentifierType;
use libtea_core::ListParams;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder().build()?;
    let service = client.service("https://api.example.com", "1.0.0")?;
    let query = ListParams::default().with_id_type(IdentifierType::ComplianceDocument);
    let page = service.query_components(&query).await?;
    println!("{} components", page.results.len());
    Ok(())
}
```

`ListParams::default()` leaves every field unset and does not require the sort type to implement `Default`. `ListParams`, `Limits`, and `PageFollowLimits` are `#[non_exhaustive]`: build them from `default()` with the `with_*` setters. Pagination is caller-managed: copy `next_page_token` into `page_token` and keep the other fields. A local HTTP origin needs `DestinationPolicy::allow_origin` for that origin only.

Environment proxies (`HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`) are honored by default. Through a proxy the DNS-based destination checks are **not** enforced, because the proxy resolves the hostname; `ClientBuilder::ignore_env_proxies(true)` connects directly and keeps them.

A match on SHA-1 checksums only is `IntegrityOutcome::Weak`, not `Verified`, and the download fails unless `ClientBuilder::allow_weak_checksums(true)` is set.

`libtea_client::url` re-exports the `url` crate used in `ClientBuilder::well_known_fetch_url`; a major-version bump of `url` is a breaking change of this crate (see the semver note in `libtea-core`).

By default the client requires an advertised API version equal to a supported one (1.0.0). `ClientBuilder::lenient_versions(true)` also accepts `1` / `1.0` and any *stable* version with the same major (pre-releases only on an exact match), for servers that advertise those forms; exact matches are always tried before same-major ones. **Risk:** the models were checked against the 1.0.0 schemas only, so a server that matches on version may still send responses this client misreads. Leave it off unless you need it.

Checksum `IntegrityOutcome::Verified` covers decoded artifact bytes only. It does not establish publisher authenticity. Detached signature and Sigstore verification are not implemented in 0.1.0; signature downloads are not cryptographically interpreted.

The workspace guide is at <https://github.com/Mehrn0ush/libtea-rs>.
