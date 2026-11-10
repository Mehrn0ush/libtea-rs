# libtea-client

Independent async Rust client for the [CycloneDX Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api).

**Independent implementation.** Not an official CycloneDX project. No conformance
certification. No cryptographic signature verification. Checksum
`IntegrityOutcome::Verified` is **not** publisher authenticity.

Implemented against Transparency Exchange API commit
`962a0228af1bfcc1232ac3f79239317ff33fdb04`. The TEA specification is still beta.

This crate depends on `libtea-core` 0.1.5. It does not depend on the CLI or the conformance crate.

```toml
[dependencies]
libtea-client = "0.1.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The snippet below uses `libtea-core` types such as `ListParams`, which `libtea-client` re-exports as `libtea_client::core` (from 0.1.5; before that, add `libtea-core` to your dependencies), and `tokio` (`#[tokio::main]`), which it does not re-export.

Edition 2021. MSRV 1.89. License Apache-2.0. TLS uses rustls. Certificate verification stays enabled. The default destination policy allows HTTPS to public addresses.

**MSRV.** The minimum supported Rust version is 1.89 for 0.1.5. Policy: the latest stable release minus about four releases; it is raised only in minor releases, and every raise is listed in the `CHANGELOG.md`. Dependencies use plain caret ranges and `Cargo.lock` is committed, so MSRV is verified with the committed lock file (`cargo +1.89.0 test --workspace --all-targets --locked`).

This snippet is compiled as a doctest but marked `no_run`, because it would contact a network host. Other examples that build are `examples/filter_by_identifier_type.rs` and `examples/paginate_components.rs`.

```rust,no_run
use libtea_client::Client;
use libtea_client::core::enums::IdentifierType;
use libtea_client::core::ListParams;

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
