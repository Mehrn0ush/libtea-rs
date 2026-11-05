# Consumer API operations

Source: `spec/openapi.yaml` at commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`, `info.version` `1.0.0`.

Paths below are relative to `{rootUrl}/v{advertised-version}`. Every operation except `POST /token` accepts an optional bearer token. `/token` uses HTTP Basic for the API-key baseline and an empty security alternative for other client-authentication methods. This SDK implements the Basic baseline.

Global error responses `400`, `401`, and `403` apply where the operation lists them. `404` uses the shared TEA error body. Clients do not treat status `404` alone as "does not exist".

## Evidence labels

Status and Tests columns use these evidence classes. They are not CycloneDX certification and not universal interoperability.

| Label | Meaning |
| --- | --- |
| **Mock / unit** | Checked-in, non-ignored `cargo test` (wiremock or unit) |
| **Oolong (ignored live)** | Checked-in ignored tests in `oolong_idtype_live` / `oolong_surface_live` against a local Oolong `tea-1.0.0` fixture |
| **Vulnetix (manual)** | Separately executed, read-only probes against `https://www.vulnetix.com/tea`; recorded in `docs/interoperability.md`; not part of default CI |
| **Not independently tested** | Method exists; no dedicated mock fixture and no recorded live assertion for that path |

A row may list more than one evidence class. Mock coverage does not imply Oolong or Vulnetix coverage, and vice versa.

| Method | Path | Operation ID | SDK method | Status | Tests |
| --- | --- | --- | --- | --- | --- |
| GET | `/discovery` | `discover` | `Service::discover_identifier`, `Client::discover` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: version selection, TEA `404` stop, non-TEA `404` failover, `401`/`403` no failover. Oolong: `discovery_get_collection_artifact_and_cle`. Vulnetix: TEI discovery |
| GET | `/products` | `queryProducts` | `Service::query_products` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual, smoke) | Mock: opaque `pageToken`, empty page, `pageSize` 101 rejected, identifier filters. Oolong: `oolong_idtype_live`; `sorted_filtered_product_continuation_keeps_query`. Vulnetix: `pageSize=1` smoke |
| GET | `/product/{uuid}` | `getProduct` | `Service::get_product` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: `gets_a_product`. Oolong: Alpha Fixture; `OBJECT_UNKNOWN`. Vulnetix: parent of known release |
| GET | `/product/{uuid}/releases` | `listProductReleases` | `Service::list_product_releases` | Implemented. Mock / unit. Oolong (ignored live) | Mock: `list_and_query_product_releases_paths_and_params`. Oolong: three versions |
| GET | `/product/{uuid}/cle` | `getProductCle` | `Service::get_product_cle` | Optional. Mock / unit. Oolong (ignored live, NotImplemented) | Mock: CLE success and `NOT_IMPLEMENTED` / `OBJECT_UNKNOWN`. Oolong: `NotImplemented` |
| GET | `/productReleases` | `queryProductReleases` | `Service::query_product_releases` | Implemented. Mock / unit. Oolong (ignored live, filter) | Mock: identifier filter in `consumer_surface`. Oolong: `oolong_idtype_live` |
| GET | `/productRelease/{uuid}` | `getProductRelease` | `Service::get_product_release` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: `product_release_get_and_unknown_identifier`. Oolong: version `1.0.0`. Vulnetix: known UUID |
| GET | `/productRelease/{uuid}/cle` | `getProductReleaseCle` | `Service::get_product_release_cle` | Optional. Mock / unit (unknown). Oolong (ignored live, NotImplemented) | Mock: `OBJECT_UNKNOWN` path in `consumer_surface`. Oolong: `NotImplemented` |
| GET | `/productRelease/{uuid}/collections` | `listProductReleaseCollections` | `Service::list_product_release_collections` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: empty intermediate page / continuation in `consumer_surface`. Oolong: `pageSize=1` continuation. Vulnetix: listing + pagination |
| GET | `/productRelease/{uuid}/collection/latest` | `getLatestProductReleaseCollection` | `Service::get_latest_product_release_collection` | Implemented. Oolong (ignored live). Vulnetix (manual) | Oolong: non-empty artifacts. Vulnetix: latest collection. Not independently mock-tested for this path alone |
| GET | `/productRelease/{uuid}/collection/{collectionVersion}` | `getProductReleaseCollectionByVersion` | `Service::get_product_release_collection` | Implemented. Oolong (ignored live). Vulnetix (manual) | Oolong: version equals latest. Vulnetix: historical v1–v4. Not independently mock-tested for this path alone |
| GET | `/components` | `queryComponents` | `Service::query_components` | Implemented. Mock / unit. Oolong (ignored live) | Mock: malformed page, type-only with `pageToken`, value-only rejected. Oolong: PURL filter |
| GET | `/component/{uuid}` | `getComponent` | `Service::get_component` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: missing JSON `Content-Type`. Oolong: Fixture Part. Vulnetix: referenced component |
| GET | `/component/{uuid}/releases` | `listComponentReleases` | `Service::list_component_releases` | Implemented. Mock / unit. Oolong (ignored live) | Mock: `list_and_query_component_releases_paths`. Oolong: surface |
| GET | `/component/{uuid}/cle` | `getComponentCle` | `Service::get_component_cle` | Optional. Mock / unit. Oolong (ignored live, NotImplemented) | Mock: `cle_success_and_object_unknown`. Oolong: `NotImplemented` |
| GET | `/componentReleases` | `queryComponentReleases` | `Service::query_component_releases` | Implemented. Mock / unit. Oolong (ignored live, filter) | Mock: opaque token with spaces / continuation. Oolong: `oolong_idtype_live` |
| GET | `/componentRelease/{uuid}` | `getComponentRelease` | `Service::get_component_release` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: `component_release_collections_and_versioned_gets`. Oolong surface. Vulnetix probe |
| GET | `/componentRelease/{uuid}/cle` | `getComponentReleaseCle` | `Service::get_component_release_cle` | Optional. Oolong (ignored live, NotImplemented) | Oolong: `NotImplemented`. Not independently mock-tested for success |
| GET | `/componentRelease/{uuid}/collections` | `listComponentReleaseCollections` | `Service::list_component_release_collections` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: page continuation. Oolong surface. Vulnetix versions 1–4 |
| GET | `/componentRelease/{uuid}/collection/latest` | `getLatestComponentReleaseCollection` | `Service::get_latest_component_release_collection` | Implemented. Mock / unit. Oolong (ignored live) | Mock + Oolong as above |
| GET | `/componentRelease/{uuid}/collection/{collectionVersion}` | `getComponentReleaseCollectionByVersion` | `Service::get_component_release_collection` | Implemented. Mock / unit. Oolong (ignored live) | Mock + Oolong as above |
| GET | `/artifact/{uuid}/latest` | `getLatestArtifact` | `Service::get_latest_artifact` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: `artifact_latest_versus_revision_and_media_type`. Oolong / Vulnetix metadata |
| GET | `/artifact/{uuid}/latest/download` | `downloadLatestArtifact` | `Service::download_latest_artifact` | Implemented. Oolong (ignored live) | Oolong: `IntegrityOutcome::Verified`. Not independently mock-tested for this alias alone |
| GET | `/artifact/{uuid}/latest/signature/download` | `downloadLatestArtifactSignature` | `Service::download_latest_signature` | Implemented. Oolong (ignored live, bytes only) | Oolong: non-empty bytes. Not cryptographic verification |
| GET | `/artifact/{uuid}/{artifactVersion}` | `getArtifactByVersion` | `Service::get_artifact` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: latest vs revision. Oolong / Vulnetix metadata |
| GET | `/artifact/{uuid}/{artifactVersion}/download` | `downloadArtifactByVersion` | `Service::download_artifact`, `Service::download_format` | Implemented. Mock / unit. Oolong (ignored live). Vulnetix (manual) | Mock: zlib/gzip/checksum/bearer strip/`304`. Oolong: hosted digests. Vulnetix: CDX/SPDX/attestation/provenance + SHA-256 |
| GET | `/artifact/{uuid}/{artifactVersion}/signature/download` | `downloadArtifactSignatureByVersion` | `Service::download_signature` | Implemented. Mock / unit. Oolong (ignored live, bytes only). Vulnetix (manual, NotPublished for probed revision) | Mock: `SIGNATURE_NOT_FOUND`, `OBJECT_UNKNOWN`, present bytes `NotDeclared`. Oolong: v2 bytes / v1 `NotPublished`. Vulnetix: `NotPublished` for the probed artifact only — not a general claim about Vulnetix signature support. Not cryptographic verification |
| POST | `/token` | `requestToken` | Internal token exchange | Implemented. Mock / unit. Oolong (ignored live limitation) | Mock: exchange after bearer `401`; coalescing under a forced two-challenge gate (`concurrent_challenges_share_one_token_exchange`). Oolong: synthetic challenge → `NOT_IMPLEMENTED` |
| HEAD | download paths | (not separately named) | `Service::head_artifact` | Implemented. Mock / unit. Oolong (ignored live) | Mock: `head_ignores_representation_content_length` and related. Oolong: `ETag` or `Content-Length` |

No other consumer paths are implemented. Publisher operations in `spec/publisher/openapi.json` are out of scope.
