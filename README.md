# TEA consumer SDK

Rust libraries for the consumer side of the [Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api).

This repository targets commit `962a0228af1bfcc1232ac3f79239317ff33fdb04` (2026-10-08). The OpenAPI document at that commit is version `1.0.0` (`OpenAPI 3.1.1`). The specification README still says "Beta 2"; this SDK follows the OpenAPI document and the normative discovery, authentication, collection, and TEI files at that commit. The baseline is [docs/specification-baseline.md](docs/specification-baseline.md).

The SDK is a client. It is not a TEA server. A response from a server does not make that server conformant, and the checks in this repository are not a conformance certificate. One local Oolong server has been exercised with this client. That is not interoperability with every TEA implementation. See [docs/interoperability.md](docs/interoperability.md).

## Crates

| Crate | Role |
| --- | --- |
| `tea-core` | TEI, version negotiation, models, checksums, destination classification. No HTTP runtime |
| `tea-client` | Async client (`tokio`, `reqwest`, `rustls`) |
| `tea-conformance` | Offline checks and a JSON report |
| `tea-cli` | `tea` binary that calls the libraries |

Version `0.1.0`. Edition 2021. MSRV 1.85. License Apache-2.0. There are no Cargo feature flags. `publish` is false, so these crates are not installed from crates.io. Build them from a checkout of this workspace.

## Build

```sh
cargo build --workspace
cargo test --workspace --all-targets
cargo run -p tea-cli -- spec
```

TLS uses rustls. Certificate verification cannot be turned off. The default destination policy allows HTTPS to public addresses and refuses cleartext, loopback, and metadata addresses. A local HTTP server needs an explicit origin exception. See [docs/security.md](docs/security.md).

Fuzz targets live in `fuzz/` and are outside the workspace. They need `cargo-fuzz` and are not part of `cargo test --workspace`.

## Compilable example

The fenced programs below are illustrations. They are not one compilable file: `?` crosses different error types, and later fences reuse names from earlier ones. The program that type-checks with this workspace is [`crates/tea-client/examples/filter_by_identifier_type.rs`](crates/tea-client/examples/filter_by_identifier_type.rs).

```sh
cargo run -p tea-client --example filter_by_identifier_type -- https://api.example.com
```

That run uses API version `1.0.0`, the default HTTPS destination policy, and `idType=COMPLIANCE_DOCUMENT` with no `idValue`. Pass `--allow-origin` only for a local HTTP origin, as the example's comments describe. It does not embed a credential.

## Discover a product release

```rust
use tea_client::Client;
use tea_core::Tei;

let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1")?;
let client = Client::builder().build()?;
let releases = client.discover(&tei).await?;
```

`discover` reads `https://example.com/.well-known/tea`, selects a mutually supported API version, and calls `/discovery`. TEI values use the `tei://` scheme.

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

`Client::service` authorizes that origin for later bearer use. The version string is the advertised API version, here `1.0.0`, which the client places in the `/v1.0.0` prefix. It is not the well-known `schemaVersion`.

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

`Verified` means every checksum declared for that format matched the decoded bytes. A mismatch, an unsupported algorithm, or a mix of matches and unsupported algorithms is `ClientError::Integrity`. An empty checksum list is `IntegrityOutcome::NotDeclared` and is not reported as verified. A matching checksum is not a detached-signature check and does not establish who published the bytes.

`ETag` is not a checksum. Pass a previously stored strong tag as `if_none_match` when you want a `304`.

## Filter a list by identifier type

```rust
use tea_client::Client;
use tea_core::enums::IdentifierType;
use tea_core::ListParams;

let client = Client::builder().build()?;
let service = client.service("https://api.example.com", "1.0.0")?;
let query = ListParams {
    page_size: None,
    page_token: None,
    sort_field: None,
    sort_order: None,
    id_type: Some(IdentifierType::ComplianceDocument),
    id_value: None,
};
let page = service.query_components(&query).await?;
```

`query_components` is the list operation. There is no `list_components` method. The same `ListParams` fields apply to the other list methods (`query_products`, `query_product_releases`, `query_component_releases`, and the per-parent release and collection lists). `ListParams::default()` is only implemented when the sort type implements `Default`. The built-in sort enums do not, so construct the struct as above.

| `id_type` | `id_value` | What the client sends |
| --- | --- | --- |
| Absent | Absent | Neither `idType` nor `idValue` |
| Present | Present | Both parameters |
| Present | Absent | `idType` only |
| Absent | Present | No request. `ListParams::validate` returns an error |

The pinned specification describes compliance-document discovery with `idType=COMPLIANCE_DOCUMENT` and no `idValue`. It does not settle a query that sets `idValue` and omits `idType`. Rejecting that combination is this SDK's behavior, not a claim that TEA 1.0 requires the rejection. The error text is `idValue requires idType`. A type without a value is still accepted.

The client does not follow `next` URLs. Copy `next_page_token` into `page_token` and call the same method again. Identifier fields you leave set are sent again with the token.

## Authentication

```rust
use tea_client::{ApiKey, Client};

let client = Client::builder()
    .api_key(ApiKey::new("key-id", "key-secret"))
    .allow_discovered_credentials(true)
    .build()?;
```

The key is sent to `POST /token` as HTTP Basic after form-encoding, with `grant_type=client_credentials`. Resource requests then use the returned bearer token. The token is not written to logs. It is not attached to artifact URLs outside the versioned API prefix, and it is removed when a redirect leaves that prefix.

Leave `allow_discovered_credentials` unset when the API key must not be posted to hosts learned from `.well-known/tea`.

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

The exception is that origin only. Other private and cleartext URLs stay refused. This does not change the default policy used by `Client::builder().build()` with no policy override.

## CLI

The binary is `tea`. It does not expose list or identifier filters. Those are library calls.

```sh
cargo run -p tea-cli -- spec
cargo run -p tea-cli -- parse-tei 'tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1'
cargo run -p tea-cli -- conformance
cargo run -p tea-cli -- discover 'tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1'
cargo run -p tea-cli -- product-release --base https://api.example.com --uuid d4d9f54a-abcf-11ee-ac79-1a52914d44b1
cargo run -p tea-cli -- download --base https://api.example.com --artifact d4d9f54a-abcf-11ee-ac79-1a52914d44b1 --revision 1 --media-type application/vnd.cyclonedx+json --output artifact.bin
```

`discover` accepts `--api-key-id`, `--api-key-secret`, `--allow-discovered-credentials`, `--well-known-url`, and `--allow-origin`. `product-release` and `download` accept `--base`, `--api-version` (default `1.0.0`), `--api-key-id`, `--api-key-secret`, and `--allow-origin`. `download` also accepts `--artifact`, `--revision` (omit it to use the latest-revision endpoint), `--media-type`, and `--output`. Without `--output`, stdout is the artifact bytes and the integrity line is written to stderr. `--api-key-id` and `--api-key-secret` must be passed together.

## Limitations

- The default supported API version is `1.0.0`. Other versions can be selected with `SupportedApi::from_versions`.
- No HTTPS SVCB lookup.
- No persistent HTTP cache.
- Signature files can be downloaded. Signature formats and trust anchors are not interpreted.
- Value-only identifier filters are rejected by this client. The specification has not defined that case.
- The CLI cannot filter lists by `idType` or `idValue`.
- Live checks so far used one local Oolong server. Other named implementations were not called.

## License

Apache-2.0. See [LICENSE](LICENSE).
