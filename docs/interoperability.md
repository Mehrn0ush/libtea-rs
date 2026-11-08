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

Third-party reports: copy `docs/interop-report-template.md`.

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

## Live Vulnetix run, 2026-11-08 (maintainer-reported)

The maintainer ran the CLI against the public Vulnetix TEA 1.0.0 deployment on
2026-11-08 and reported the following. The agent that wrote this section did
not run it and has not seen the raw output; the sanitized result file below is
**not yet in the repository**.

| Field | Value |
| --- | --- |
| Operations | `discover`, then `download` |
| Artifact | `fbe825ce-1fe6-58b2-bf8d-25fdd47b5f13`, revision `1` |
| Bytes | 5728 |
| Integrity | reported as verified (declared checksums matched the decoded bytes; that is not publisher authenticity) |
| ETag | strong |
| Sanitized probe result file | **PLACEHOLDER**: `docs/interop/vulnetix-2026-11-08.json` (to be added from `scripts/interop-probe.sh`; see `HANDOFF-COMMANDS.md`) |

Nothing else about this run is recorded. In particular no response timings,
header dumps, or other endpoints were reported, so none are claimed.

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
