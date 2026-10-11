# TEA consumer SDK

Rust libraries for the consumer side of the [Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api).

This repository implements the specification at commit `962a0228af1bfcc1232ac3f79239317ff33fdb04` on `main` (2026-10-08). The OpenAPI document at that commit is version `1.0.0` (`OpenAPI 3.1.1`). The specification README still says "Beta 2"; the OpenAPI `info.version` is the contract used here. The baseline, including known gaps, is in [docs/specification-baseline.md](docs/specification-baseline.md).

The SDK is a client. It does not claim that a server is conformant because the server answered, and the local conformance report does not claim interoperability with another implementation.

## Crates

| Crate | Role |
| --- | --- |
| `tea-core` | TEI, version negotiation, models, checksums, destination classification. No HTTP runtime |
| `tea-client` | Async client (`tokio`, `reqwest`, `rustls`) |
| `tea-conformance` | Offline checks and a JSON report |
| `tea-cli` | `tea` binary that calls the libraries |

## Requirements

- Rust 1.85 or newer (the MSRV declared in the workspace)
- Edition 2021
- Default features are enough for a consumer. TLS uses rustls. Certificate verification cannot be disabled

```sh
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo doc --workspace --no-deps
```

Fuzz targets are in `fuzz/` and are outside the workspace:

```sh
cargo install cargo-fuzz
cargo fuzz run tei_parse
```

## Discover a product release

```rust
use tea_client::Client;
use tea_core::Tei;

let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1")?;
let client = Client::builder().build()?;
let releases = client.discover(&tei).await?;
```

`discover` reads `https://example.com/.well-known/tea`, selects a mutually supported API version, and calls `/discovery`. The default destination policy allows HTTPS to public addresses and refuses cleartext, loopback, and metadata addresses.

## Read a release from a known API base

```rust
use tea_client::Client;
use tea_core::ProductReleaseId;

let client = Client::builder().build()?;
let service = client.service("https://api.example.com", "1.0.0")?;
let id = ProductReleaseId::parse("d4d9f54a-abcf-11ee-ac79-1a52914d44b1")?;
let release = service.get_product_release(id).await?;
let collection = service
    .get_latest_product_release_collection(id)
    .await?;
```

## Download an artifact and check it

```rust
use tea_client::{ArtifactFetch, Client};
use tea_core::{ArtifactId, IntegrityOutcome};

let service = client.service("https://api.example.com", "1.0.0")?;
let artifact = service
    .get_artifact(ArtifactId::parse("d4d9f54a-abcf-11ee-ac79-1a52914d44b1")?, 1)
    .await?;
let fetched = service
    .download_format(&artifact, "application/vnd.cyclonedx+json", None)
    .await?;
if let ArtifactFetch::Body(body) = fetched {
    assert_eq!(body.integrity.outcome, IntegrityOutcome::Verified);
}
```

`Verified` is returned only when every checksum declared for that format matched the decoded bytes. A mismatch, an unsupported algorithm, or a mix of matches and unsupported algorithms is `ClientError::Integrity`. An empty checksum list is `IntegrityOutcome::NotDeclared` and is not reported as verified. Prefer `download_format`, which uses the checksums on that format.

`ETag` is not a checksum. Pass a previously stored strong tag as `if_none_match` when you want a `304`.

## Filter a list by identifier type

```rust
use tea_core::enums::IdentifierType;
use tea_core::ListParams;

let mut query = ListParams::default();
query.id_type = Some(IdentifierType::ComplianceDocument);
let page = service.query_components(&query).await?;
```

`idType` may be sent without `idValue`. Setting both sends `idType` and `idValue`. Setting `idValue` without `idType` is rejected before the request. The pinned specification does not settle that value-only case, so the rejection is SDK behavior pending upstream clarification, not a claim that TEA 1.0 forbids it.

## Authentication

```rust
use tea_client::{ApiKey, Client};

let client = Client::builder()
    .api_key(ApiKey::new("key-id", "key-secret"))
    .allow_discovered_credentials(true)
    .build()?;
```

The key is sent to `POST /token` as HTTP Basic after form-encoding, with `grant_type=client_credentials`. Resource requests then use the returned bearer token. The token is not written to logs. It is not attached to artifact URLs outside the versioned API prefix, and it is removed when a redirect leaves that prefix.

Leave `allow_discovered_credentials` unset when the API key must not be posted to hosts learned from `.well-known/tea`. `Client::service` authorizes the origin of the base URL you pass.

## Local servers

```rust
use tea_client::Client;
use tea_core::DestinationPolicy;
use url::Url;

let origin = Url::parse("http://127.0.0.1:8080")?;
let client = Client::builder()
    .destination_policy(DestinationPolicy::allow_origin(&origin)?)
    .build()?;
```

The exception is that origin only. Other private and cleartext URLs stay refused.

## CLI

```sh
cargo run -p tea-cli -- spec
cargo run -p tea-cli -- parse-tei 'tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1'
cargo run -p tea-cli -- conformance
cargo run -p tea-cli -- discover 'tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1'
```

## Limitations

- Supported API version in the default client is `1.0.0`. Other versions can be selected with `SupportedApi::from_versions`.
- No HTTPS SVCB lookup.
- No persistent HTTP cache.
- Signature files can be downloaded. Signature formats and trust anchors are not interpreted.
- Live interoperability was not run. See [docs/interoperability.md](docs/interoperability.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
