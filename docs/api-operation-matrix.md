# Consumer API operations

Source: `spec/openapi.yaml` at commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`, `info.version` `1.0.0`.

Paths below are relative to `{rootUrl}/v{advertised-version}`. Every operation except `POST /token` accepts an optional bearer token. `/token` uses HTTP Basic for the API-key baseline and an empty security alternative for other client-authentication methods. This SDK implements the Basic baseline.

Global error responses `400`, `401`, and `403` apply where the operation lists them. `404` uses the shared TEA error body. Clients do not treat status `404` alone as "does not exist".

Status distinguishes evidence. **Mock-tested** means a `cargo test` wiremock or unit test. **Live Oolong** means the ignored `oolong_idtype_live` test, or the 2026-10-10 local session that is not in the workspace. Neither label means the operation is conformant, and neither was repeated against another server. Shared helpers are not coverage of every path.

| Method | Path | Operation ID | SDK method | Status | Tests |
| --- | --- | --- | --- | --- | --- |
| GET | `/discovery` | `discover` | `Service::discover_identifier`, `Client::discover` | Implemented. Mock-tested. One local session, not a workspace test | Mock: version selection, TEA `404` stop, non-TEA `404` failover, `401` no failover, `403` no failover and no token fetch. The 2026-10-10 Oolong session resolved one fixture TEI. That program is not in `cargo test` |
| GET | `/products` | `queryProducts` | `Service::query_products` | Implemented. Mock-tested. Live Oolong (ignored test) | Mock: opaque `pageToken`, empty page, `pageSize` 101 rejected before the request, paired `idType`/`idValue`, absent filters omitted. Live: type-only page, continuation with the filter kept, paired PURL value, unfiltered five-product list |
| GET | `/product/{uuid}` | `getProduct` | `Service::get_product` | Implemented + verified | `gets_a_product`; `406` is `ClientError::Api` |
| GET | `/product/{uuid}/releases` | `listProductReleases` | `Service::list_product_releases` | Implemented + unverified | Uses `page`. No fixture for this path |
| GET | `/product/{uuid}/cle` | `getProductCle` | `Service::get_product_cle` | Optional; partially verified | `NOT_IMPLEMENTED` is `CleFetch::NotImplemented`. Success and `OBJECT_UNKNOWN` are not fixtures |
| GET | `/productReleases` | `queryProductReleases` | `Service::query_product_releases` | Implemented. Live Oolong for one filter. Not mock-tested for this path | Ignored live test: `idType=PURL` returned the one release that has a PURL (`1.0.0`), and the unfiltered list was larger. No wiremock fixture for this JSON shape |
| GET | `/productRelease/{uuid}` | `getProductRelease` | `Service::get_product_release` | Implemented. Not a workspace test | A 2026-10-10 local SDK session read one release from Oolong. That program is not in `cargo test` |
| GET | `/productRelease/{uuid}/cle` | `getProductReleaseCle` | `Service::get_product_release_cle` | Optional; unverified | Same `cle` helper as product CLE |
| GET | `/productRelease/{uuid}/collections` | `listProductReleaseCollections` | `Service::list_product_release_collections` | Implemented + unverified | Uses `page` |
| GET | `/productRelease/{uuid}/collection/latest` | `getLatestProductReleaseCollection` | `Service::get_latest_product_release_collection` | Implemented + unverified | Uses `get` |
| GET | `/productRelease/{uuid}/collection/{collectionVersion}` | `getProductReleaseCollectionByVersion` | `Service::get_product_release_collection` | Implemented + unverified | Uses `get` |
| GET | `/components` | `queryComponents` | `Service::query_components` | Implemented. Mock-tested. Live Oolong (ignored test) | Mock: malformed page is `ClientError::Json`; `idType` without `idValue` is sent and kept with `pageToken`; value-only returns `ClientError::Model` and sends no request. Live: `idType=PURL` returned only the component that has a PURL |
| GET | `/component/{uuid}` | `getComponent` | `Service::get_component` | Implemented + partially verified | Missing `Content-Type` (`text/plain`) is `ClientError::Model` |
| GET | `/component/{uuid}/releases` | `listComponentReleases` | `Service::list_component_releases` | Implemented + unverified | Uses `page` |
| GET | `/component/{uuid}/cle` | `getComponentCle` | `Service::get_component_cle` | Optional; unverified | Uses `cle` |
| GET | `/componentReleases` | `queryComponentReleases` | `Service::query_component_releases` | Implemented. Live Oolong for one filter. Not mock-tested for this path | Ignored live test: `idType=PURL` returned Fixture Part `1.0.0` only. The unfiltered list was larger. No wiremock fixture for this JSON shape |
| GET | `/componentRelease/{uuid}` | `getComponentRelease` | `Service::get_component_release` | Implemented + unverified | Uses `get` |
| GET | `/componentRelease/{uuid}/cle` | `getComponentReleaseCle` | `Service::get_component_release_cle` | Optional; unverified | Uses `cle` |
| GET | `/componentRelease/{uuid}/collections` | `listComponentReleaseCollections` | `Service::list_component_release_collections` | Implemented + unverified | Uses `page` |
| GET | `/componentRelease/{uuid}/collection/latest` | `getLatestComponentReleaseCollection` | `Service::get_latest_component_release_collection` | Implemented + unverified | Uses `get` |
| GET | `/componentRelease/{uuid}/collection/{collectionVersion}` | `getComponentReleaseCollectionByVersion` | `Service::get_component_release_collection` | Implemented + unverified | Uses `get` |
| GET | `/artifact/{uuid}/latest` | `getLatestArtifact` | `Service::get_latest_artifact` | Implemented + unverified | Uses `get` |
| GET | `/artifact/{uuid}/latest/download` | `downloadLatestArtifact` | `Service::download_latest_artifact` | Implemented + unverified | Uses `download_url` |
| GET | `/artifact/{uuid}/latest/signature/download` | `downloadLatestArtifactSignature` | `Service::download_latest_signature` | Implemented + unverified | Uses `signature_api` |
| GET | `/artifact/{uuid}/{artifactVersion}` | `getArtifactByVersion` | `Service::get_artifact` | Implemented + partially verified | CLI and download fixtures parse artifact JSON. No dedicated error status for this path |
| GET | `/artifact/{uuid}/{artifactVersion}/download` | `downloadArtifactByVersion` | `Service::download_artifact`, `Service::download_format` | Implemented. Mock-tested. One local session, not a workspace test | Mock: zlib `deflate` then SHA-256; gzip; checksum mismatch; bearer stripped off the API prefix; `304`. The 2026-10-10 Oolong session matched one hosted artifact's SHA-256. That program is not in `cargo test` |
| GET | `/artifact/{uuid}/{artifactVersion}/signature/download` | `downloadArtifactSignatureByVersion` | `Service::download_signature` | Implemented + verified | `SIGNATURE_NOT_FOUND`, `OBJECT_UNKNOWN`, present bytes reported `NotDeclared` |
| POST | `/token` | `requestToken` | Internal token exchange | Implemented + verified | One exchange after a bearer `401`. Discovery `403` does not call it |

`HEAD` is described for download operations when a server supports it. `Service::head_artifact` sends `HEAD` and returns headers. It is not a substitute for `GET`.

No other consumer paths are implemented. Publisher operations in `spec/publisher/openapi.json` are out of scope.
