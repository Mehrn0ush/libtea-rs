# libtea-rs

Independent Rust client libraries for the [Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api) (TEA).

This repository targets commit `962a0228af1bfcc1232ac3f79239317ff33fdb04` (2026-10-08). The OpenAPI document at that commit is version `1.0.0` (`OpenAPI 3.1.1`). The specification README still says "Beta 2"; this project follows the OpenAPI document and the normative discovery, authentication, collection, and TEI files at that commit. The baseline is [docs/specification-baseline.md](docs/specification-baseline.md).

These crates are a TEA **consumer** (client). They are not a TEA server, not an official CycloneDX release, and not a conformance certificate. Interoperability evidence to date is one local Oolong fixture and one public Vulnetix TEA 1.0.0 deployment. That is not universal compatibility. See [docs/interoperability.md](docs/interoperability.md).

## Crates

| Crate | Role |
| --- | --- |
| `libtea-core` | TEI, version negotiation, models, checksums, destination classification. No HTTP runtime |
| `libtea-client` | Async client (`tokio`, `reqwest`, `rustls`) |
| `libtea-conformance` | Offline checks and a JSON report (`publish = false`) |
| `libtea-cli` | `libtea` binary that calls the libraries (`publish = false`) |

Version `0.1.0`. Edition 2021. MSRV 1.85. License Apache-2.0. There are no Cargo feature flags. Publish `libtea-core` first, then `libtea-client`. The CLI stays unpublished for now. See [CHANGELOG.md](CHANGELOG.md).

These packages are independent libraries, not CycloneDX-owned packages.

## Build and test

```sh
cargo build --workspace --locked
cargo test --workspace --all-targets --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo package -p libtea-core --locked
```

TLS uses rustls. Certificate verification cannot be turned off. The default destination policy allows HTTPS to public addresses and refuses cleartext, loopback, and metadata addresses. A local HTTP server needs an explicit origin exception (`--allow-origin` / `DestinationPolicy::allow_origin`). See [docs/security.md](docs/security.md).

## Verify (CLI and examples)

Offline / local smoke:

```sh
cargo run -p libtea-cli --locked -- spec
cargo run -p libtea-cli --locked -- parse-tei 'tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1'
cargo run -p libtea-cli --locked -- conformance
```

`filter_by_identifier_type` example (destination policy and TEI input):

```sh
# Fake host — expect a transport error (no server).
cargo run -p libtea-client --locked --example filter_by_identifier_type -- https://api.example.com

# TEI form (resolves via discovery when the host is reachable).
cargo run -p libtea-client --locked --example filter_by_identifier_type -- \
  tei://vulnetix.com/uuid/71286087-babf-5795-a6ce-4c350bc944b4

# Cleartext / loopback refused by default — expect "URL scheme is not allowed: http".
cargo run -p libtea-client --locked --example filter_by_identifier_type -- http://localhost:3005
cargo run -p libtea-client --locked --example filter_by_identifier_type -- http://127.0.0.1:3005

# Local Oolong (or other loopback TEA) — allow that origin explicitly.
cargo run -p libtea-client --locked --example filter_by_identifier_type -- \
  --allow-origin http://127.0.0.1:3005 \
  http://127.0.0.1:3005
```

Public Vulnetix TEA 1.0.0 (network; read-only smoke):

```sh
cargo run -p libtea-cli --locked -- discover --help
cargo run -p libtea-cli --locked -- discover \
  'tei://vulnetix.com/uuid/71286087-babf-5795-a6ce-4c350bc944b4'

cargo run -p libtea-cli --locked -- product-release --help
cargo run -p libtea-cli --locked -- product-release \
  --base 'https://www.vulnetix.com/tea' \
  --api-version '1.0.0' \
  --uuid '71286087-babf-5795-a6ce-4c350bc944b4'

cargo run -p libtea-cli --locked -- download \
  --base 'https://www.vulnetix.com/tea' \
  --api-version '1.0.0' \
  --artifact 'fbe825ce-1fe6-58b2-bf8d-25fdd47b5f13' \
  --revision 1 \
  --media-type 'application/vnd.cyclonedx+json' \
  --output './vulnetix-sbom.cdx.json'
```

A successful download prints `integrity verified`. The output file is a local artifact; do not commit it (it is gitignored). Live results are evidence for that deployment only — see [docs/interoperability.md](docs/interoperability.md).

## Workflow A — TEI → collection → artifact integrity

```rust
use libtea_client::{ArtifactFetch, Client};
use libtea_core::{IntegrityOutcome, Tei};

let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1")?;
let client = Client::builder().build()?;
let releases = client.discover(&tei).await?;
let selected = &releases[0];
let service = client.service(&selected.root_url, &selected.api_version)?;
let collection = service
    .get_latest_product_release_collection(selected.product_release)
    .await?;
let artifact = &collection.artifacts[0];
let fetched = service
    .download_format(artifact, &artifact.formats[0].media_type, None)
    .await?;
if let ArtifactFetch::Body(body) = fetched {
    assert_eq!(body.integrity.outcome, IntegrityOutcome::Verified);
}
```

`Verified` means every checksum declared for that format matched the **decoded** artifact bytes (after `gzip` / HTTP `deflate`). It is not publisher authenticity and not signature verification. Version `0.1.0` does not implement cryptographic signature verification, publisher authenticity checks, or Sigstore trust-policy evaluation. Detached signature bytes can be downloaded; they are not interpreted.

## Workflow B — collection history and pagination

Pagination is caller-managed. Keep the same `ListParams` and replace only `page_token`. The client does not follow `next` URLs and does not auto-loop on a repeated token.

```rust
use libtea_core::ListParams;

let mut query = ListParams {
    page_size: Some(2),
    ..ListParams::default()
};
let first = service
    .list_product_release_collections(release_id, &query)
    .await?;
query.page_token = first.next_page_token.clone();
let second = service
    .list_product_release_collections(release_id, &query)
    .await?;
let historical = service
    .get_product_release_collection(release_id, 1)
    .await?;
```

## Workflow C — API key → bearer → authenticated request

Prefer environment variables for secrets. Do not put secrets in shell history when avoidable.

```sh
export TEA_API_KEY_ID=your-key-id
export TEA_API_KEY_SECRET=your-key-secret
```

```rust
use libtea_client::{ApiKey, Client};

let client = Client::builder()
    .api_key(ApiKey::new("key-id", "key-secret"))
    .allow_discovered_credentials(true) // only when discovery hosts may receive the key
    .build()?;
```

The key is sent to `POST /token` as HTTP Basic after form-encoding, with `grant_type=client_credentials`. Token refresh is serialized with a **client-wide** async mutex: waiters may reuse a bearer another request just cached when that value differs from the one that received `401`. That often yields one exchange for overlapping challenges on the same origin, but failed exchanges, expired or missing cache entries, and challenges for other origins can each require their own `POST /token`. Because the lock is client-wide, refreshes for unrelated origins may wait on each other. Tokens are not logged. `Debug` for `ApiKey` redacts the secret.

## Compilable examples

The fenced programs above are illustrations. Programs that type-check in this workspace:

- [`crates/tea-client/examples/filter_by_identifier_type.rs`](crates/tea-client/examples/filter_by_identifier_type.rs)
- [`crates/tea-client/examples/paginate_components.rs`](crates/tea-client/examples/paginate_components.rs)
- [`crates/tea-core/examples/parse_tei.rs`](crates/tea-core/examples/parse_tei.rs)
- [`crates/tea-core/examples/select_endpoint.rs`](crates/tea-core/examples/select_endpoint.rs)

```sh
cargo run -p libtea-client --locked --example paginate_components -- https://api.example.com
cargo run -p libtea-core --locked --example parse_tei
cargo run -p libtea-core --locked --example select_endpoint
```

## CLI reference

```sh
cargo run -p libtea-cli --locked -- spec --json
cargo run -p libtea-cli --locked -- parse-tei 'tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1'
cargo run -p libtea-cli --locked -- conformance
cargo run -p libtea-cli --locked -- discover 'tei://example.com/uuid/…'
cargo run -p libtea-cli --locked -- product-release --base https://api.example.com --uuid …
cargo run -p libtea-cli --locked -- collections --base https://api.example.com --uuid … --page-size 2 --json
cargo run -p libtea-cli --locked -- collections --base https://api.example.com --uuid … --latest --json
cargo run -p libtea-cli --locked -- download --base https://api.example.com --artifact … --revision 1 --output artifact.bin
```

**Credentials precedence:** complete `--api-key-id` + `--api-key-secret` pair (secret on argv is **insecure**), else complete `TEA_API_KEY_ID` + `TEA_API_KEY_SECRET`, else anonymous. Partial pairs and mixing CLI id with env secret are errors. JSON modes write structured data to stdout; diagnostics go to stderr.

## Integrity versus authenticity

| Layer | What this library does |
| --- | --- |
| Transport (TLS) | Enforced (rustls); no verify bypass |
| Content integrity | Declared checksums over decoded bytes |
| Publisher authenticity | Not established by checksum match |
| Detached signature / Sigstore | Download only; no crypto verification in 0.1.0 |

## Versioning policy

`0.1.0` is the first public library release and should be treated as unstable. Breaking changes may occur before `1.0.0`. Pin exact versions in Cargo.toml if you need a freeze.

## Limitations

- Default supported API version is `1.0.0`.
- No HTTPS SVCB lookup; no persistent HTTP cache.
- Signature formats and trust anchors are not interpreted.
- Value-only identifier filters are rejected (`idValue requires idType`).
- CLI is intentionally thin; list identifier filters remain library calls.
- `libtea-conformance` is offline smoke, not official certification.

## License

Apache-2.0. See [LICENSE](LICENSE).
