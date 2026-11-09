# Interoperability

A server that answers this SDK is not, by that fact, a conforming TEA server. This SDK is not a conformance certificate. Results below are evidence for **two named deployments** (Oolong local fixture, Vulnetix public) only. They are **not** ecosystem-wide TEA compatibility.

Pinned contract: CycloneDX/transparency-exchange-api commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`, OpenAPI `1.0.0` (`spec/PINNED.md`).

## Repeatable read-only probe

```bash
TEA_TEI='tei://vulnetix.com/uuid/71286087-babf-5795-a6ce-4c350bc944b4' \
TEA_BASE_URL='https://www.vulnetix.com/tea' \
TEA_PRODUCT_RELEASE='71286087-babf-5795-a6ce-4c350bc944b4' \
TEA_ARTIFACT_UUID='fbe825ce-1fe6-58b2-bf8d-25fdd47b5f13' \
TEA_MEDIA_TYPE='application/vnd.cyclonedx+json' \
./scripts/interop-probe.sh
```

Writes timestamped JSON/Markdown under `interop-results/` (gitignored) including
date, client version, spec commit, server base URL, operations, pass/fail, and
checksum outcome. Credentials, if any, come from `TEA_API_KEY_ID` /
`TEA_API_KEY_SECRET` and are never printed.

Optional CI: `.github/workflows/interop-manual.yml` (`workflow_dispatch` only;
never required for merge).

Third-party reports: open an issue with the server name and version, the date, the commit of this repository, and the commands run; do not include credentials.

## How to run the live Oolong suite

Start the fixture profile (Oolong sources are not modified by this SDK):

```bash
cd /path/to/oolong/oolong-server
export PORT=3005
export SERVER_HOST=http://127.0.0.1:3005
export API_VERSION=v1.0.0
export CONTENT_DIR=/path/to/oolong/fixtures/tea-1.0.0
export PAGE_TOKEN_SECRET=oolong-local-interop
export TEA_SPEC_COMMIT=962a0228af1bfcc1232ac3f79239317ff33fdb04
node dist/main.js
```

Then, from this SDK repository:

```bash
cargo test -p libtea-client --locked --test oolong_idtype_live -- --ignored --nocapture
cargo test -p libtea-client --locked --test oolong_surface_live -- --ignored --nocapture
```

`cargo test` without `--ignored` does not run these files. CI does not call external TEA servers.

## Checked-in live evidence — Oolong (2026-11-04)

Against Oolong on `127.0.0.1:3005` with `fixtures/tea-1.0.0` and the pinned TEA commit above.

### Identifier filters (`oolong_idtype_live`)

| Scenario | Result |
| --- | --- |
| Type-only product filter | `idType=PURL`, `pageSize=1` returned Alpha Fixture, `hasNext` true, non-empty token; no `idValue` |
| Continuation | Same filter and page size kept; Beta Fixture returned |
| Type and value | Only Beta Fixture for the paired PURL |
| Value only | No HTTP request; SDK error `idValue requires idType` |
| Unfiltered products | Alpha, Beta, Delta, Epsilon, Gamma |
| Type-only on all four list endpoints | Filtered lists were smaller than unfiltered lists |

### Surface coverage (`oolong_surface_live`)

| Capability | Asserted result | Test |
| --- | --- | --- |
| Well-known + `Client::discover` | Fixture TEI resolves to API `1.0.0` | `discovery_get_collection_artifact_and_cle` |
| Nested product / component / releases / collections | Fixture graph navigated; pagination kept query params | same |
| Artifact download | `IntegrityOutcome::Verified` on declared digests | same |
| Signature download | Bytes present or `NotPublished`. **Not** cryptographic verification | same |
| CLE | `CleFetch::NotImplemented` (**Oolong limitation**) | same |
| `POST /token` | Synthetic challenge → Oolong `NOT_IMPLEMENTED` (**Oolong limitation**) | `token_endpoint_not_implemented_is_server_limitation` |

Checksum verification means the decoded bytes match digests the server declared for that format. It does not mean a publisher signature was checked.

## Independent public evidence — Vulnetix TEA 1.0.0

A committed result file, [`docs/interop/vulnetix-2026-11-08.json`](interop/vulnetix-2026-11-08.json), records one run of `scripts/interop-probe.sh` against this deployment (anonymous, read-only GETs; see below). The two operation tables under it come from earlier manual probes and **have no committed result file**: they are reported, not reproducible from this repository. All of it is evidence for that one deployment only.

| Field | Value |
| --- | --- |
| API root | `https://www.vulnetix.com/tea` |
| TEA version exercised | `1.0.0` |
| Product Release UUID | `71286087-babf-5795-a6ce-4c350bc944b4` |
| TEI | `tei://vulnetix.com/uuid/71286087-babf-5795-a6ce-4c350bc944b4` |
| Artifact UUID (CycloneDX) | `fbe825ce-1fe6-58b2-bf8d-25fdd47b5f13` revision `1` |
| Expected SHA-256 | `fa0b6b174d3c3f55dcd0e0fa917fbbe86d1799cf2f423bc0765bd4997a4e370a` |
| Auth | Anonymous (no API key exercised against Vulnetix) |

### Operations reported

| Operation | Result |
| --- | --- |
| TEI Discovery | PASS |
| Product Release retrieval | PASS |
| Latest Collection | PASS |
| Historical Collections v1–v4 | PASS |
| Collection listing | PASS |
| Collection pagination (`pageSize` 1 and 2) | PASS — opaque tokens copied unchanged; no missing/duplicate versions |
| CycloneDX SBOM download + SHA-256 | PASS (5728 bytes) |
| Strong ETag observed | PASS |

### Further read-only SDK probes

| Operation | Result | Classification |
| --- | --- | --- |
| `get_product` (parent of known release) | PASS | Interop |
| `get_component` / `get_component_release` | PASS | Interop |
| `list_component_release_collections` | PASS (versions 1–4) | Interop |
| Artifact metadata latest vs explicit revision | PASS | Interop |
| SPDX BOM / attestation / Sigstore provenance download + SHA-256 | PASS | Content integrity only |
| Signature download for the probed artifact revision | `SignatureFetch::NotPublished` | Observation for that operation only: no signature was published for the requested artifact through the path tested. This does **not** show that Vulnetix lacks signature support in general, and it is not an SDK defect |
| `query_products` `pageSize=1` | PASS (`has_next`) | Smoke |

Downloading a `provenance.sigstore.json` (or similar) and verifying its checksum proves **content integrity**, not Sigstore cryptographic verification of the signed subject.

## Probe run on the public deployment (2026-11-08)

`scripts/interop-probe.sh` was run once against the public deployment on 2026-11-08 (client 0.1.0, spec commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`). The output is committed as [`docs/interop/vulnetix-2026-11-08.json`](interop/vulnetix-2026-11-08.json).

| Field | Value |
| --- | --- |
| Operations | `discover`, `product-release`, `download`: 3 passed, 0 failed |
| Artifact | `fbe825ce-1fe6-58b2-bf8d-25fdd47b5f13`, media type `application/vnd.cyclonedx+json` |
| Auth | Anonymous |
| Checksum result | `checksum_match_decoded_bytes`: every declared checksum matched the decoded bytes (not publisher authenticity) |

The file records no response timings, headers, byte counts or other endpoints, so none are claimed from it.

## Implementations named by the specification

| Name | Role described there | What this repository verified |
| --- | --- | --- |
| py-libtea | Python library | Not executed |
| pypi-tea | Service in front of PyPI | Not executed |
| ReARM CLI / server | Client / server | Not executed |
| Vulnetix | Client named in the specification's implementations list; public API root above | Public Consumer probes as above (TEA 1.0.0 paths) |
| Oolong | Server | Local fixture profile |
| sbomify | Server | Not executed |

Some servers advertise API versions that are not SemVer, such as `1` or `1.0`. By default this client requires an advertised version equal to a supported one and rejects such strings before calling `/discovery`. `ClientBuilder::lenient_versions(true)` also accepts `1` and `1.0`, and any stable version with the same major (see the `libtea-client` README). No server is judged by this; it describes the client's choice.

## Behavior that can affect other implementations

- Query values follow the OpenAPI example and encode `/` as `%2F`.
- Opaque `pageToken` values are sent unchanged (including values that look like URLs).
- Pagination continuation is primarily caller-managed: keep the same `ListParams` and replace only `page_token`. Opt-in `follow_pages` can follow tokens with `max_pages` / `max_items` and a repeated-token guard; it is not the default path.
- Unknown TEA error codes on `/discovery` do not fail over. `401` and `403` do not fail over.
- HTTP and non-public addresses are refused unless that origin is explicitly allowed.
- Checksums are calculated after `gzip` or zlib-wrapped HTTP `deflate` is removed.
- Token refresh uses one async lock per API origin. Waiters may reuse a suitable cached bearer or share the failure of the exchange they queued behind; other origins are not blocked. See `docs/auth.md` for when more exchanges happen.

## Manual interop workflow (`.github/workflows/interop-manual.yml`)

The workflow is `workflow_dispatch` only and has no URL or TEI input. Every
target comes from the protected **`tea-interop`** GitHub Environment. If that
Environment is configured as documented below (required reviewers, deployment
branches limited to `main`, no admin bypass, the URL as an Environment
variable), whoever dispatches the run cannot send the API key to a host of
their choosing. The repository cannot enforce those settings: they live in
GitHub, and without them a writer could dispatch a modified workflow from
another branch.
Top-level `permissions: {}`; the job has `contents: read` only, and checkout
does not persist credentials. The CLI is built in a step that has **no**
secrets (`cargo` runs third-party build scripts); the API key reaches exactly
one step, which runs the prebuilt binary and never a build tool, and
`scripts/interop-probe.sh` refuses to run `cargo` at all when credentials are
set and `TEA_CLI_BIN` is not. Dispatch inputs are limited to two optional UUIDs
(product release, artifact), which the CLI parses as UUIDs and appends to the
fixed base URL.

What the run probes comes from these, and at least one of the three must be
set or the job stops early with an `::error::` that lists them:

| What | Dispatch input | Environment variable |
| --- | --- | --- |
| TEI to discover | none (by design) | `TEA_TEI` |
| Product release UUID | `product_release` | `TEA_PRODUCT_RELEASE` |
| Artifact UUID | `artifact_uuid` | `TEA_ARTIFACT_UUID` |

A dispatch input wins over the Environment variable of the same name. A TEI
cannot be passed as an input (`gh workflow run -f tei=...` fails with HTTP 422,
"Unexpected inputs"). Example, with `TEA_BASE_URL` set in `tea-interop`:

```sh
gh workflow run interop-manual.yml --repo Mehrn0ush/libtea-rs --ref main \
  -f product_release=71286087-babf-5795-a6ce-4c350bc944b4
gh run list --repo Mehrn0ush/libtea-rs --workflow interop-manual.yml --limit 1
```

Take the run id from that list, approve the `tea-interop` deployment in the
GitHub UI, then `gh run watch <run-id> --exit-status` and
`gh run download <run-id> --dir interop-run`.

Required Environment settings (repository **Settings → Environments →
`tea-interop`**):

| Setting | Value |
| --- | --- |
| Required reviewers | At least one maintainer other than the dispatcher; enable "Prevent self-review" |
| Deployment branches and tags | Selected branches: `main` only |
| Allow administrators to bypass | Off |
| Environment variable `TEA_BASE_URL` | Required. `https://` API root of the deployment under test; the job fails if it is unset or not `https://` |
| Environment variables `TEA_TEI`, `TEA_PRODUCT_RELEASE`, `TEA_ARTIFACT_UUID`, `TEA_MEDIA_TYPE` | Optional defaults |
| Environment secrets `TEA_API_KEY_ID`, `TEA_API_KEY_SECRET` | Optional. Set them only on the Environment, not as repository secrets, so no other workflow can read them |

Result files are uploaded as the `interop-results` artifact for 14 days. They
contain no credentials (`crates/tea-cli/tests/interop_probe.rs`).

### Probe exit-status contract

`scripts/interop-probe.sh` reads `libtea download --require-checksums` by exit
status only:

| CLI status | Probe `checksum_result` | Meaning |
| --- | --- | --- |
| 0 | `checksum_match_decoded_bytes` | Checksums were declared and all matched the decoded bytes. SHA-1-only matches count only with `TEA_ALLOW_WEAK_CHECKSUMS=1` (recorded as `allow_weak_checksums: true`) |
| 4 | `weak_algorithms_only` | Every declared checksum matched but all are SHA-1 |
| other | `not_confirmed` | Any other failure |

## Offline conformance suite and mock tests

`libtea-conformance` runs checks that use the pinned specification examples and rules. The checks do not contact a network.

```sh
cargo test -p libtea-conformance
cargo run -p libtea-cli -- conformance
```

The report includes the specification commit and version and one result per case. A passing report means those local checks passed. It is not a statement that this SDK is a conforming TEA server, and it is not a statement that any particular server is conformant.

### What the static suite covers

- Registry TEI examples parse, validate, and keep their spelling
- Case variants of a TEI are not equal
- The normative well-known example selects the higher-priority equal version
- An empty discovery array is rejected
- An unknown `error` string is still recognized as a TEA error
- A wrong SHA-256 is not reported as verified

### What the HTTP tests cover

The default `libtea-client` tests use a local mock server. They cover production destination refusal, JSON size limits, a product read, TEA `404` recognition, failover from a non-TEA `404`, one API-key exchange after a bearer challenge, redirect stripping of `Authorization`, checksum verification of decoded bytes, and identifier query serialization (type only, both parameters, neither parameter, and value-only rejection before a request).

Two ignored live tests, `oolong_idtype_live` and `oolong_surface_live`, talk to a local Oolong process. They are not part of `cargo test`; see "How to run the live Oolong suite" above.

### What is not claimed

No public TEA server was used as a conformance oracle. Passing the mock tests, or the ignored Oolong test, does not imply interoperability with ReARM, pypi-tea, sbomify, or any other implementation. A checksum match is not detached-signature verification.

## Requirements matrix

Requirements are taken from the pinned revision. Identifiers are local to this SDK. "Client" means a TEA consumer. Server-only obligations are listed only where they change client behavior.

| ID | Source | Requirement | Client | Module | Verification | Test | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TEA-TEI-001 | discovery/readme.md, tei-types | TEI form `tei://domain/type/id`; registry syntax for registered types | Mandatory | `libtea_core::Tei` | Unit and conformance | `parses_registry_examples`, `TEA-TEI-001` | Implemented |
| TEA-TEI-002 | discovery/readme.md "Comparing TEIs" | Equality is the original character sequence. Clients shall not rewrite a TEI | Mandatory | `libtea_core::Tei` | Unit and conformance | `does_not_treat_case_variants_as_equal` | Implemented |
| TEA-TEI-003 | discovery/readme.md | Unique identifier is one segment and shall not contain `/` or `%2F` | Mandatory | `libtea_core::Tei` | Unit | `rejects_slash_and_port` | Implemented |
| TEA-TEI-004 | discovery/readme.md | Domain has no port. Scheme and type are lowercase | Mandatory | `libtea_core::Tei` | Unit | `rejects_slash_and_port` | Implemented |
| TEA-DISC-001 | discovery/readme.md, well-known schema | Select the highest mutually supported SemVer version, then priority. Omitted priority is 1. Build metadata is ignored for comparison and kept for the path | Mandatory | `libtea_core::discovery` | Unit and conformance | `selects_the_normative_well_known_example`, `keeps_build_metadata_spelling` | Implemented |
| TEA-DISC-002 | discovery/readme.md | Successful `/discovery` is a non-empty array. Empty `200` is not success | Mandatory | `libtea_core::DiscoveryInfo` | Unit | `empty_discovery_array_is_rejected` | Implemented |
| TEA-DISC-003 | discovery/readme.md | TEA `404` (`application/json` object with string `error`) stops discovery and is not failover, including unknown error codes | Mandatory | `libtea_client::Client::discover` | HTTP test | `discovery_follows_version_and_stops_on_tea_404` | Implemented |
| TEA-DISC-004 | discovery/readme.md | A non-TEA `404`, `5xx`, or unusable body is a failed attempt and the next compatible endpoint is tried | Mandatory | `libtea_client::Client::discover` | HTTP test | `non_tea_404_fails_over_to_the_next_endpoint` | Implemented |
| TEA-DISC-005 | discovery/readme.md | `401` and `403` do not fail over | Mandatory | `libtea_client::Client::discover` | HTTP test | `bearer_401_does_not_fail_over`, `discovery_403_does_not_fail_over_or_fetch_a_token`; non-TEA `404` still fails over | Verified |
| TEA-DISC-006 | discovery/readme.md | Exactly one of `tei` and `purl` | Mandatory | `Service::discover_identifier` | API check | Rejects both and neither before the request | Implemented |
| TEA-AUTH-001 | auth/readme.md | API key is HTTP Basic on `POST /token` with `grant_type=client_credentials`, after form-encoding each part | Mandatory when a key is configured | `libtea_client` token exchange | HTTP test | `bearer_challenge_exchanges_an_api_key_once` | Implemented |
| TEA-AUTH-002 | auth/readme.md, discovery/readme.md | Resource calls use `Authorization: Bearer`. The token is not inspected | Mandatory | `libtea_client` | HTTP test | Bearer retry sends the token and not the API key | Implemented |
| TEA-AUTH-003 | RFC 6750 as cited | `invalid_token` or a bearer challenge may refresh once per request | Optional recovery, implemented | `libtea_client` | HTTP test | One token request | Implemented |
| TEA-AUTH-004 | discovery/readme.md | Do not send the access token to another origin or outside the API base. Do not send it to external artifact URLs | Mandatory | `libtea_client` download and redirect handling | HTTP test | `bearer_token_is_kept_under_the_prefix_and_stripped_after_that` (real token), `redirect_to_another_origin_is_refused_before_a_credential_is_sent` | Verified |
| TEA-AUTH-005 | auth/readme.md | Do not probe `/token` to discover whether authentication exists | Mandatory | Client starts with the resource request | Design | No startup token request | Implemented |
| TEA-AUTH-006 | discovery/readme.md | Credentials are not sent to discovered hosts unless the caller opts in | SDK security policy | `ClientBuilder::allow_discovered_credentials` | Default is false | Builder default | Implemented |
| TEA-ART-001 | tea-collection.md, OpenAPI checksum | Checksums cover bytes after transfer and content coding are removed. A mismatch is not success | Mandatory | `libtea_core::verify_checksums`, `ContentDecoder` | Unit and HTTP | `sha256_matches_and_mismatch_is_not_success`, `zlib_deflate_is_decoded_before_the_checksum`, gzip artifact test | Verified |
| TEA-ART-002 | tea-collection.md | Unsupported algorithms are reported and are not treated as verified | Mandatory | `IntegrityOutcome` | Unit | `missing_and_unknown_algorithms_are_not_verified` | Implemented |
| TEA-ART-003 | OpenAPI artifact download | `ETag` is not a checksum. Weak tags are not strong byte identity. `304` has no body | Mandatory | `DownloadedArtifact::strong_etag`, `ArtifactFetch::NotModified` | HTTP test | Weak `ETag` is flagged; `conditional_get_304_is_not_a_checksum_and_has_no_body` | Verified |
| TEA-ART-004 | OpenAPI | `url` on a format requires checksums. External URLs are fetched without a bearer token | Mandatory | `ArtifactFormat::validate`, download path | Unit and HTTP | `artifact_with_url_requires_checksum` | Implemented |
| TEA-HTTP-001 | discovery/readme.md, auth/readme.md | TLS verification stays on. Default policy refuses `http`, loopback, link-local, and private addresses | Mandatory | `DestinationPolicy`, rustls | Unit and HTTP | `production_policy_rejects_cleartext_and_metadata` | Implemented |
| TEA-HTTP-002 | SDK security policy | Resolved addresses are filtered. An exception is one scheme, host, and port. IPv4 embedded in mapped, 6to4, and NAT64 addresses is classified. `0.0.0.0/8` and `198.18.0.0/15` are not public | Mandatory | `classify_ip`, `allows_resolved_ip` | Unit | `classifies_embedded_private_addresses`, `exception_is_scheme_host_and_port` | Verified |
| TEA-HTTP-003 | SDK limits | Discovery, JSON, and artifact bodies are size-capped | Mandatory | `Limits` | HTTP test | `oversized_json_is_rejected` | Implemented |
| TEA-MODEL-001 | OpenAPI schemas | Product, release, component, collection, artifact, CLE, pagination, and errors keep their distinct types | Mandatory | `libtea_core::model` | Unit | `preserves_unknown_artifact_type` | Implemented |
| TEA-LIST-001 | OpenAPI list parameters, compliance-document discovery | `idType` may be sent without `idValue`. Both parameters are sent when both are set. Neither is sent when both are absent | Client behavior for a use case the pin describes. Not a rule that every server must accept type-only filters | `ListParams::validate`, `Service::page` | Unit, mock HTTP, and an ignored live test | `id_type_without_id_value_is_accepted`, `type_only_identifier_filter_is_serialized`, `both_identifier_filters_are_encoded_in_the_query`, `absent_identifier_filters_are_omitted`, `oolong_idtype_live` (ignored; local Oolong) | Implemented. Mock tests cover serialization. The ignored live test covers products, product releases, components, and component releases. `idValue` without `idType` is rejected before the request with `idValue requires idType`. The pinned specification does not settle the value-only case |
| TEA-MODEL-002 | doc/tea-uuid-scope.md | UUID newtypes are not interchangeable | Mandatory | `libtea_core::ids` | Types | Distinct id types | Implemented |
| TEA-CLE-001 | OpenAPI `404` text | `NOT_IMPLEMENTED` on a CLE endpoint is an absent capability, not a missing parent | Mandatory | `CleFetch` | HTTP test | `cle_not_implemented_is_distinct_from_an_unknown_object` | Partially verified. Product CLE `NOT_IMPLEMENTED` is tested. A successful CLE document and component CLE are not |
| TEA-SIG-001 | OpenAPI signature download | `SIGNATURE_NOT_FOUND` means the revision exists and has no signature. `OBJECT_UNKNOWN` does not. Downloaded signature bytes are not reported as cryptographically verified | Mandatory | `SignatureFetch` | HTTP test | `signature_absence_is_not_verification_and_unknown_is_an_error`, `signature_bytes_are_not_reported_as_verified` | Verified for those outcomes. No signature algorithm is checked, because the pinned OpenAPI does not require consumer cryptographic verification |

Server requirements that are not client behavior, including how a server assigns UUIDs and which update-reason precedence it uses when publishing, are not implemented as client obligations.

## API operation matrix

Source: vendored OpenAPI at `spec/vendor/openapi.yaml` (commit
`962a0228af1bfcc1232ac3f79239317ff33fdb04`, `info.version` `1.0.0`). See
`spec/PINNED.md`.

Paths below are relative to `{rootUrl}/v{advertised-version}`. Every operation
except `POST /token` accepts an optional bearer token. `/token` uses HTTP Basic
for the API-key baseline. This SDK implements the Basic baseline.

Global error responses `400`, `401`, and `403` apply where the operation lists
them. `404` uses the shared TEA error body. Clients do not treat status `404`
alone as "does not exist".

### Evidence labels

| Label | Meaning |
| --- | --- |
| **Mock / unit** | Checked-in, non-ignored `cargo test` (wiremock or unit) |
| **Oolong (ignored live)** | Ignored tests in `oolong_idtype_live` / `oolong_surface_live` |
| **Vulnetix (manual)** | Read-only probes; `docs/interoperability.md` + `scripts/interop-probe.sh` |
| **Untested (reason)** | Explicit gap with rationale |

The **Tested** column lists independent mock/unit test names that exercise the
path. Live evidence is additional, not a substitute for Mock when Mock is
claimed. Mock coverage is not CycloneDX certification and not universal
interoperability.

| Method | Path | Operation ID | SDK method | Status | Tested |
| --- | --- | --- | --- | --- | --- |
| GET | `/discovery` | `discover` | `Service::discover_identifier`, `Client::discover` | Implemented. Mock / unit. Oolong. Vulnetix | `discovery_follows_version_and_stops_on_tea_404`; `non_tea_404_fails_over_to_the_next_endpoint`; `bearer_401_does_not_fail_over`; `discovery_403_does_not_fail_over_or_fetch_a_token` |
| GET | `/products` | `queryProducts` | `Service::query_products` | Implemented. Mock / unit. Oolong. Vulnetix (smoke) | `product_pages_keep_tokens_opaque_and_do_not_follow_urls`; `empty_page_and_malformed_page_and_oversized_page_size`; `type_only_identifier_filter_is_serialized`; `value_only_identifier_filter_sends_no_request`; `follow_pages_multi_page_then_exhausted` |
| GET | `/product/{uuid}` | `getProduct` | `Service::get_product` | Implemented. Mock / unit. Oolong. Vulnetix | `gets_a_product`; `oversized_json_is_rejected` |
| GET | `/product/{uuid}/releases` | `listProductReleases` | `Service::list_product_releases` | Implemented. Mock / unit. Oolong | `list_and_query_product_releases_paths_and_params` |
| GET | `/product/{uuid}/cle` | `getProductCle` | `Service::get_product_cle` | Optional. Mock / unit. Oolong (`NotImplemented`) | `cle_success_for_every_cle_endpoint`; `cle_not_implemented_is_distinct_from_an_unknown_object`; `cle_success_and_object_unknown` |
| GET | `/productReleases` | `queryProductReleases` | `Service::query_product_releases` | Implemented. Mock / unit. Oolong | `list_and_query_product_releases_paths_and_params` (query path); identifier filters in `protocol` |
| GET | `/productRelease/{uuid}` | `getProductRelease` | `Service::get_product_release` | Implemented. Mock / unit. Oolong. Vulnetix | `product_release_get_and_unknown_identifier` |
| GET | `/productRelease/{uuid}/cle` | `getProductReleaseCle` | `Service::get_product_release_cle` | Optional. Mock / unit. Oolong (`NotImplemented`) | `cle_success_for_every_cle_endpoint`; `cle_success_and_object_unknown` (unknown path) |
| GET | `/productRelease/{uuid}/collections` | `listProductReleaseCollections` | `Service::list_product_release_collections` | Implemented. Mock / unit. Oolong. Vulnetix | `pagination_anomalies_are_visible_to_callers` |
| GET | `/productRelease/{uuid}/collection/latest` | `getLatestProductReleaseCollection` | `Service::get_latest_product_release_collection` | Implemented. Mock / unit. Oolong. Vulnetix | `product_release_collection_latest_and_by_version` |
| GET | `/productRelease/{uuid}/collection/{collectionVersion}` | `getProductReleaseCollectionByVersion` | `Service::get_product_release_collection` | Implemented. Mock / unit. Oolong. Vulnetix | `product_release_collection_latest_and_by_version` |
| GET | `/components` | `queryComponents` | `Service::query_components` | Implemented. Mock / unit. Oolong | `empty_page_and_malformed_page_and_oversized_page_size`; `type_only_filter_stays_on_the_next_page`; `value_only_identifier_filter_sends_no_request` |
| GET | `/component/{uuid}` | `getComponent` | `Service::get_component` | Implemented. Mock / unit. Oolong. Vulnetix | `get_component_happy_path`; `not_acceptable_is_an_api_error_and_a_missing_json_type_is_rejected` (missing JSON Content-Type) |
| GET | `/component/{uuid}/releases` | `listComponentReleases` | `Service::list_component_releases` | Implemented. Mock / unit. Oolong | `list_and_query_component_releases_paths` |
| GET | `/component/{uuid}/cle` | `getComponentCle` | `Service::get_component_cle` | Optional. Mock / unit. Oolong (`NotImplemented`) | `cle_success_for_every_cle_endpoint`; `cle_success_and_object_unknown` (`NOT_IMPLEMENTED` path) |
| GET | `/componentReleases` | `queryComponentReleases` | `Service::query_component_releases` | Implemented. Mock / unit. Oolong | `list_and_query_component_releases_paths` |
| GET | `/componentRelease/{uuid}` | `getComponentRelease` | `Service::get_component_release` | Implemented. Mock / unit. Oolong. Vulnetix | `component_release_collections_and_versioned_gets` |
| GET | `/componentRelease/{uuid}/cle` | `getComponentReleaseCle` | `Service::get_component_release_cle` | Optional. Mock / unit. Oolong (`NotImplemented`) | `cle_success_for_every_cle_endpoint` |
| GET | `/componentRelease/{uuid}/collections` | `listComponentReleaseCollections` | `Service::list_component_release_collections` | Implemented. Mock / unit. Oolong. Vulnetix | `component_release_collections_and_versioned_gets` |
| GET | `/componentRelease/{uuid}/collection/latest` | `getLatestComponentReleaseCollection` | `Service::get_latest_component_release_collection` | Implemented. Mock / unit. Oolong | `component_release_collection_latest_and_by_version`; `component_release_collections_and_versioned_gets` |
| GET | `/componentRelease/{uuid}/collection/{collectionVersion}` | `getComponentReleaseCollectionByVersion` | `Service::get_component_release_collection` | Implemented. Mock / unit. Oolong | `component_release_collection_latest_and_by_version`; `component_release_collections_and_versioned_gets` |
| GET | `/artifact/{uuid}/latest` | `getLatestArtifact` | `Service::get_latest_artifact` | Implemented. Mock / unit. Oolong. Vulnetix | `artifact_latest_versus_revision_and_media_type` |
| GET | `/artifact/{uuid}/latest/download` | `downloadLatestArtifact` | `Service::download_latest_artifact` | Implemented. Mock / unit. Oolong | `download_latest_artifact_and_signature_aliases` |
| GET | `/artifact/{uuid}/latest/signature/download` | `downloadLatestArtifactSignature` | `Service::download_latest_signature` | Implemented. Mock / unit. Oolong (bytes only) | `download_latest_artifact_and_signature_aliases` — **not** cryptographic verification |
| GET | `/artifact/{uuid}/{artifactVersion}` | `getArtifactByVersion` | `Service::get_artifact` | Implemented. Mock / unit. Oolong. Vulnetix | `artifact_latest_versus_revision_and_media_type` |
| GET | `/artifact/{uuid}/{artifactVersion}/download` | `downloadArtifactByVersion` | `Service::download_artifact`, `download_format` | Implemented. Mock / unit. Oolong. Vulnetix | `zlib_deflate_is_decoded_before_the_checksum` (200 on the API path); `artifact_checksum_covers_decoded_bytes_and_not_a_redirect_with_credentials` (API path answers 302; 200 after redirect); `conditional_get_304_is_not_a_checksum_and_has_no_body`; `redirect_limit_and_gzip_expansion_limit_are_enforced` |
| GET | `/artifact/{uuid}/{artifactVersion}/signature/download` | `downloadArtifactSignatureByVersion` | `Service::download_signature` | Implemented. Mock / unit. Oolong (bytes). Vulnetix (`NotPublished` for probed revision) | `signature_absence_is_not_verification_and_unknown_is_an_error`; `signature_bytes_are_not_reported_as_verified` — **not** cryptographic verification |
| POST | `/token` | `requestToken` | Internal token exchange | Implemented. Mock / unit | `client_credentials_uses_basic_and_a_form_body`; `rejected_bearer_is_dropped_after_one_retry`; `concurrent_n32_challenges_one_token_exchange`; `token_error_text_omits_the_secret`; `hanging_token_endpoint_times_out_without_the_secret`; `hanging_token_origin_does_not_stall_waiters_or_other_origins`; `zero_lifetime_is_not_presented`; `short_lifetime_is_still_presented`; unit `expiry_margin_does_not_consume_a_short_lifetime` |
| HEAD | download paths | (not separately named) | `Service::head_artifact` | Implemented. Mock / unit. Oolong | `head_ignores_representation_content_length`; `head_without_content_length_returns_metadata`; `head_bearer_challenge_is_an_authentication_error` |

#### Cross-cutting negatives (not a single operation)

| Claim | Tested |
| --- | --- |
| Oversized JSON body rejected | `oversized_json_is_rejected` |
| Unknown enum / error strings preserved | `preserves_unknown_artifact_type` (core); `unknown_error_is_still_a_tea_error` (conformance) |
| Pagination token round-trip (opaque) | `product_pages_keep_tokens_opaque_and_do_not_follow_urls`; `type_only_filter_stays_on_the_next_page` |
| Value-only identifier filter rejected pre-HTTP | `value_only_identifier_filter_sends_no_request` |
| `follow_pages` limits / loop guard | `follow_pages_multi_page_then_exhausted`; `follow_pages_empty_page_exhausts`; `follow_pages_hits_max_items`; `follow_pages_repeated_token_guard`; unit `stops_on_repeated_token`; `respects_max_pages`; `empty_next_token_is_exhausted_not_followed` |

No other consumer paths are implemented. Publisher operations are out of scope.

#### Remaining caveats (still true)

- Signature download tests assert bytes / absence only — not authenticity.
- Oolong CLE and `/token` live results are server limitations, not SDK defects.
- Vulnetix evidence is for that named deployment only.
