# Interoperability

A server that answers this SDK is not, by that fact, a conforming TEA server. This SDK is not a conformance certificate. Results below are evidence for named deployments only. They are not universal TEA compatibility.

Pinned contract: CycloneDX/transparency-exchange-api commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`, OpenAPI `1.0.0`.

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

| Field | Value |
| --- | --- |
| API root | `https://www.vulnetix.com/tea` |
| TEA version exercised | `1.0.0` |
| Product Release UUID | `71286087-babf-5795-a6ce-4c350bc944b4` |
| TEI | `tei://vulnetix.com/uuid/71286087-babf-5795-a6ce-4c350bc944b4` |
| Artifact UUID (CycloneDX) | `fbe825ce-1fe6-58b2-bf8d-25fdd47b5f13` revision `1` |
| Expected SHA-256 | `fa0b6b174d3c3f55dcd0e0fa917fbbe86d1799cf2f423bc0765bd4997a4e370a` |
| Auth | Anonymous (no API key exercised against Vulnetix) |

### Previously reported (accepted as prior evidence; not re-hammered in every audit)

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

### Newly exercised on 2026-11-05 (targeted read-only SDK probes)

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

## Implementations named by the specification

| Name | Role described there | What this repository verified |
| --- | --- | --- |
| py-libtea | Python library | Not executed |
| pypi-tea | Service in front of PyPI | Not executed |
| ReARM CLI / server | Client / server | Not executed |
| Vulnetix | Client note in older docs; public API root above | Public Consumer probes as above (TEA 1.0.0 paths) |
| Oolong | Server | Local fixture profile |
| sbomify | Server | Not executed |

The pinned implementations note also links `https://us.sbom.cybeats.com/.well-known/tea`. A fetch on 2026-10-10 returned `versions: ["1"]`. This client parses API versions as SemVer and would reject `1` before calling `/discovery`.

## Behavior that can affect other implementations

- Query values follow the OpenAPI example and encode `/` as `%2F`.
- Opaque `pageToken` values are sent unchanged (including values that look like URLs).
- Pagination continuation is the caller's responsibility: keep the same `ListParams` and replace only `page_token`. The SDK does not auto-paginate and does not break loops on a repeated token.
- Unknown TEA error codes on `/discovery` do not fail over. `401` and `403` do not fail over.
- HTTP and non-public addresses are refused unless that origin is explicitly allowed.
- Checksums are calculated after `gzip` or zlib-wrapped HTTP `deflate` is removed.
- Token refresh uses a client-wide async mutex. Waiters may reuse a suitable cached bearer; overlapping challenges often share one exchange, but failed exchanges or other origins can require more. The lock may briefly serialize refreshes across unrelated origins.
