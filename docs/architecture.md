# Architecture

```text
libtea-cli
  -> libtea-conformance (offline checks and reports)
  -> libtea-client     (HTTP, auth, streaming, retries)
       -> libtea-core  (TEI, versions, models, checksums, policy)
```

`libtea-core` has no Tokio, reqwest, filesystem, or process-global state. Parsing and validation can be used by another HTTP stack.

`libtea-client` owns the HTTP session, destination policy, token cache, and resource limits. `Service` is one versioned API base. `Client::discover` walks well-known endpoints and calls `/discovery` on each selected base.

`libtea-conformance` runs checks that do not contact a network and writes a JSON report. A passing report means the local checks passed. It does not certify a server.

`libtea-cli` calls the public crates. It does not reimplement TEI parsing or HTTP. Its commands are `spec`, `parse-tei`, `conformance`, `discover`, `product-release`, `collections`, `download`, `sync`, `diff` and `cache`. Identifier filters are `ListParams` on `libtea-client` list methods; the CLI does not expose them.

## Request path

1. Parse the TEI and keep the original string.
2. Build `https://{domain}/.well-known/tea` from the domain spelling. The destination policy accepts or refuses that URL before a connection.
3. Parse the well-known document. Select mutually supported versions by SemVer precedence, then priority.
4. `GET {root}/v{advertised}/discovery?tei=...`. A TEA error stops the walk. A non-TEA failure tries the next selected endpoint.
5. Typed JSON models are validated after decoding. Unknown forward-compatible values are preserved.
6. Artifact downloads stream through a size cap, remove one layer of `Content-Encoding` (`gzip`, `deflate`, or `identity`), and hash the decoded bytes. The bearer token is attached only when the URL is under the versioned API prefix.

## Extensibility

- TEI types outside the registry parse and remain valid. Registry checks apply only to known types.
- Artifact types, update reasons, and other OpenAPI enums keep unknown strings.
- API versions other than `1.0.0` are selected when the caller lists them in `SupportedApi`.
- Checksum algorithms outside the registry are `ChecksumAlgorithm::Other` and produce `IntegrityOutcome::Unsupported`.

Well-known documents and discovery entries accept unknown fields at every level (tested in `crates/tea-core/tests/discovery_tolerance.rs`), although the upstream schemas set `additionalProperties: false`; the client is deliberately tolerant of what servers send. The lock file and cache entries are our own formats and reject unknown fields.

## Timeouts

Three limits bound a request (`Limits`):

| Limit | Default | Applies to |
| --- | --- | --- |
| `request_timeout` | 30 s | The whole of a request that is not an artifact download: discovery, JSON metadata, the token exchange. Headers and body. |
| `download_timeout` | 30 min | The whole of an artifact or signature `GET`, body included. |
| `idle_timeout` | 30 s | Any single wait for the next bytes of a response, for every request. |

`connect_timeout` (10 s) bounds connecting. An artifact that keeps arriving is
therefore not cut off after 30 s, which a single total timeout would do; a
transfer that stalls is cut off after `idle_timeout`, and no download runs longer
than `download_timeout`. Raise `download_timeout` for very large artifacts on
slow links. `idle_timeout` and `download_timeout` must not be zero
(`ClientBuilder::build` fails). Retries (`max_retries`) start a new attempt with
a fresh clock.

## Dependency direction

`libtea-core` does not depend on `libtea-client`. `libtea-client` does not depend on `libtea-cli` or `libtea-conformance`. The fuzz crate is excluded from the workspace so `cargo test --workspace` does not require `cargo-fuzz`.

## Specification baseline

This SDK implements the consumer side of the Transparency Exchange API from one pinned revision. It does not follow the specification forward automatically.

| Field | Value |
| --- | --- |
| Repository | https://github.com/CycloneDX/transparency-exchange-api |
| Branch | `main` |
| Commit | `962a0228af1bfcc1232ac3f79239317ff33fdb04` |
| Commit date | 2026-10-08T06:28:26Z |
| Retrieval date | 2026-10-10 |
| Commit subject | Merge pull request #410 from Mehrn0ush/docs/readme-bom-exchange-link |
| Specification version (`info.version`) | `1.0.0` |
| OpenAPI version | `3.1.1` |
| ECMA context repository | https://github.com/Ecma-TC54/ECMA-xxx-TEA |

The OpenAPI document at that commit is vendored as `spec/vendor/openapi.yaml` (upstream path `spec/openapi.yaml`). Its `info.version` is `1.0.0`. The repository README still says "Beta 2". This SDK treats the OpenAPI document and the normative discovery, authentication, collection, and TEI registry files at this commit as the contract, and records the README status line as stale. See the section "Where the specification is ambiguous" below.

### Normative source files

Paths are relative to the upstream repository at the pinned commit. Vendored under `spec/vendor/`: `openapi.yaml`, `discovery/readme.md`, `discovery/tea-well-known.schema.json`, `auth/readme.md`, `tea-collection/tea-collection.md` and `tei-types/README.md`. The other files are read at the pinned commit but not copied here.

- `spec/openapi.yaml` — consumer operations, schemas, and HTTP response metadata
- `discovery/readme.md` and `ecma-doc/discovery.md` — TEI syntax, well-known retrieval, version negotiation, failover, and authentication interaction (these two files match at this commit)
- `discovery/tea-well-known.schema.json` — `.well-known/tea` schema
- `auth/readme.md` — API key exchange and bearer usage
- `tei-types/*.md` — TEI type registry
- `tea-collection/tea-collection.md` — collection and artifact byte rules
- `doc/tea-uuid-scope.md` — UUID scope
- `api-flow/consumer.md` — informative consumer flows

### Informative documents

- `README.md`
- `doc/tea-requirements.md`
- `doc/tea-usecases.md`
- `doc/tea-implementations.md`
- `signatures/signature.md` — explicitly not part of the TEA standard
- `spec/publisher/openapi.json` — publisher API, out of consumer scope

### Known implementation limitations

- HTTPS SVCB records are not queried. The discovery document allows them when a client is known to support them. This client uses `A`/`AAAA` resolution through the system resolver and then the HTTPS URL from the TEI domain.
- The persistent HTTP cache is opt-in and experimental (`docs/cache.md`); by default nothing is cached. Callers can send `If-None-Match`. `Cache-Control` is honoured only for `no-store` and `private`; `max-age` does not skip revalidation.
- Detached signature bytes are downloaded. Signature technology is not interpreted. `signatures/signature.md` describes a `/trust-anchors/` resource that is not in the OpenAPI document, so it is not implemented.
- Optional OAuth profiles other than the API-key `client_credentials` baseline (SAML, JWT bearer, token exchange, mutual TLS) are not implemented.
- CLE is retrieved as the TEA projection. This client does not fetch or validate a full ECMA-428 document.
- The SDK is not `no_std` and is not a TEA server.
- List filters accept `idType` alone and both `idType` and `idValue`. `idValue` without `idType` is rejected before the request with `idValue requires idType`. The pinned text does not define that value-only case.
- The CLI has no identifier-filter options; those are library calls (`ListParams`).

## Where the specification is ambiguous

Pinned revision: `962a0228af1bfcc1232ac3f79239317ff33fdb04` (retrieved 2026-10-10).

### README status versus OpenAPI version

The repository README says the standard is "Beta 2". The upstream `spec/openapi.yaml` has `info.version` `1.0.0`, and the discovery examples advertise `"1.0.0"`.

This SDK implements OpenAPI `1.0.0` from the pinned commit. The README status line is treated as documentation drift, not as an instruction to implement an older beta.

### Error codes are closed in OpenAPI and open in the discovery prose

`error-response` lists a closed `unknown-error-type` enum and sets `additionalProperties: false`. The discovery document says a client shall not reject a `404` because it does not know the `error` string, so that a later revision can extend the enum without turning those responses into failover triggers.

Interpretation used: recognition follows the prose. A JSON object with a string `error` property and `Content-Type: application/json` is a TEA error even when the code is unknown or extra properties are present. Unknown codes are preserved. They do not cause failover on `/discovery`.

Consequence: a server that adds a field next to `error` is still treated as terminal for discovery. A strict schema validator would reject that body and might fail over. This client does not.

### Failover order

Endpoint selection must prefer the highest mutually supported SemVer version, then the higher `priority`. The failover section says the client should preserve priority order from highest to lowest while choosing the next untried compatible endpoint.

Interpretation used: the attempt list is sorted by version precedence descending, then priority descending, then document order. Failover walks that list. Priority does not override a higher mutually supported version.

### Query encoding of `tei` and `purl`

The discovery prose example leaves `/` unencoded in the `tei` query value (`tei%3A//products.example.com/...`). The OpenAPI description's example percent-encodes the whole identifier (`tei%3A%2F%2F...`) and says clients apply query serialization once. Servers must decode once.

Interpretation used: the client percent-encodes every octet that is not unreserved, once, which matches the OpenAPI example. A server that decodes the query component once accepts both spellings. A server that decodes twice would corrupt an OpenAPI-style value.

### Versioned base versus OpenAPI server URL

OpenAPI paths such as `/discovery` and `/token` are relative to a server URL that already includes `/v1.0.0` in the example server entry. The discovery document says to append `/v` and the advertised version string to `rootUrl`.

Interpretation used: `rootUrl` has no version prefix. Operation URLs are `{rootUrl}/v{advertised}/` + the OpenAPI path. `+` in build metadata is left unencoded in that path.

### Collection and artifact version minima

The prose says collection and artifact versions start at 1. The schemas type those fields as integers and do not declare `minimum: 1`.

Interpretation used: zero and negative values are rejected as non-conforming.

### HTTP in well-known documents

The JSON Schema pattern allows `http` so local documents still validate. The discovery prose says a client shall reject an `http` base unless that specific base was explicitly configured for local testing.

Interpretation used: documents may parse with `http` URLs. The default destination policy refuses to connect to them. `DestinationPolicy::allow_origin` permits one configured origin.

### Trailing dot on TEI domains

Normative examples use a DNS name without a trailing root dot. Comparison is exact, so `example.com` and `example.com.` would be different spellings if both were accepted.

Interpretation used: a trailing dot is rejected. This is recorded because the prose does not say so in those words.

### Identifier list filters

TEA 1.0 describes compliance-document discovery with `idType=COMPLIANCE_DOCUMENT` and no `idValue`. This SDK sends `idType` alone, and it sends both parameters when both are set. Neither parameter is a claim that every server must accept a type-only query. It is the filter this client is able to send, matching that described use case.

A request that sets `idValue` and omits `idType` is rejected before any HTTP call. The pinned specification does not settle that value-only case. The rejection is this SDK's current behavior. It is not a mandatory TEA 1.0 requirement.

The error text is `idValue requires idType`. The check rejects only a value without a type. Type-only input is accepted. The pinned specification still does not define the value-only case.

### Signatures document

`signatures/signature.md` says it is work in progress and not part of the TEA standard. It mentions `/trust-anchors/`, which is absent from `spec/vendor/openapi.yaml`.

Interpretation used: signature download endpoints from the OpenAPI document are implemented. Trust-anchor distribution is not.

## Dependencies

The lock file (`Cargo.lock`) is committed. TLS is rustls only. Direct dependencies and why each is there:

| Crate | Used by | Why |
| --- | --- | --- |
| `serde`, `serde_json` | core, client, conformance | Models, lock file, cache entries, JSON output. |
| `thiserror` | core, client | Error types. |
| `url` | core, client, cli | URL parsing and destination checks. |
| `uuid`, `semver` | core | Typed identifiers and API version precedence. |
| `sha1`, `sha2`, `sha3`, `blake2`, `blake3`, `digest` | core | The checksum algorithms a server may declare, behind one interface. |
| `base64` | core, client | TEI/purl encoding and the HTTP Basic credential. |
| `flate2` | core, client | Removing `gzip` and `deflate` before hashing. |
| `tokio` | client, cli | Async runtime; its `fs` feature is used for streaming downloads to a temporary file. |
| `reqwest` | client | HTTP with rustls; certificate verification stays on. |
| `futures-util`, `bytes` | client | Streaming response bodies. |
| `tracing` | client | Log events with URLs and credentials redacted. |
| `clap`, `anyhow` | cli | Command-line parsing and error reporting. |
| `proptest`, `wiremock` | tests only | Property tests and local mock servers. |

`cargo-cyclonedx` is installed only in the release workflow's SBOM job. It is not a dependency of any crate. The release and Action scripts use Bash, `curl` and the Python 3 standard library. The fuzz crate under `fuzz/` has its own lock file.
