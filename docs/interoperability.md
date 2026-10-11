# Interoperability

A server that answers this SDK is not, by that fact, a conforming TEA server. This SDK is not a conformance certificate. Results below are for one local Oolong server that implements the pinned contract. They were not repeated against ReARM, sbomify, pypi-tea, Vulnetix, Cybeats, or py-libtea.

Pinned contract: CycloneDX/transparency-exchange-api commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`, OpenAPI `1.0.0`.

## Checked-in live tests

`crates/tea-client/tests/oolong_idtype_live.rs` is ignored. `cargo test` does not run it. It uses `cdx-tea-client` against a local Oolong process that is already listening. On 2026-10-11 those tests passed:

| Scenario | What was observed |
| --- | --- |
| Type-only product filter | `idType=PURL` and `pageSize=1` returned Alpha Fixture, `hasNext` true, and a non-empty `nextPageToken`. `idValue` was not sent |
| Pagination | The next call kept `idType=PURL` and `pageSize=1`, sent that token, and returned Beta Fixture with `hasNext` false |
| Type and value | `idType=PURL` and `idValue=pkg:generic/oolong-fixture/beta` returned only Beta Fixture |
| Value only | No request was sent. The client rejected the combination. The error text is now `idValue requires idType`. The 2026-10-11 run observed the previous wording |
| No identifier filter | The product list was Alpha, Beta, Delta, Epsilon, and Gamma, with `hasNext` false |
| Other lists | Type-only `idType=PURL` on products, product releases, components, and component releases returned only objects that carry a PURL. The unfiltered lists were larger |

Those tests check filtering. They do not discover a TEI, download an artifact, or verify a checksum.

## Earlier local session, not a workspace test

On 2026-10-10 a separate program, using this SDK and not retained in the workspace, called the same local Oolong profile. That session observed:

- well-known discovery and `/discovery` for a fixture TEI
- retrieval of a known product release
- download of a hosted artifact whose SHA-256 matched an independent digest of the bytes

That program is not `cargo test`. Re-running the ignored filter tests does not re-run those three calls.

## Implementations named by the specification

The pinned `doc/tea-implementations.md` names other software. This SDK was not run against them.

| Name | Role described there | What this repository verified |
| --- | --- | --- |
| py-libtea | Python library | Not executed |
| pypi-tea | Service in front of PyPI | Not executed |
| ReARM CLI | Command line client | Not executed |
| Vulnetix | Client, described as implementing TEA v0.4.0 | Not executed. That claim is a different baseline from OpenAPI 1.0.0 |
| Oolong | Server | Local server only, as above. The public demo host was not used |
| ReARM server | Server | Not executed |
| sbomify | Server | Not executed |

The pinned implementations note also links `https://us.sbom.cybeats.com/.well-known/tea`. A fetch on 2026-10-10 returned `versions: ["1"]`. This client parses API versions as SemVer and would reject `1` before calling `/discovery`. `Client::discover` was not pointed at that host.

## Behavior that can affect other implementations

- Query values follow the OpenAPI example and encode `/` as `%2F`.
- The version path uses the advertised string, including build metadata, with `+` left unencoded.
- Unknown TEA error codes on `/discovery` do not fail over. `401` and `403` do not fail over.
- HTTP and non-public addresses are refused unless that origin is explicitly allowed.
- Checksums are calculated after `gzip` or zlib-wrapped HTTP `deflate` is removed.
