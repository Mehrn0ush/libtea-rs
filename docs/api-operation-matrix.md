# Consumer API operations

Source: `spec/openapi.yaml` at commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`, `info.version` `1.0.0`.

Paths below are relative to `{rootUrl}/v{advertised-version}`. Every operation except `POST /token` accepts an optional bearer token. `/token` uses HTTP Basic for the API-key baseline and an empty security alternative for other client-authentication methods. This SDK implements the Basic baseline.

Global error responses `400`, `401`, and `403` apply where the operation lists them. `404` uses the shared TEA error body. Clients do not treat status `404` alone as "does not exist".

Status is after the 2026-10-10 remediation tests. "Verified" means a happy path and a relevant error or schema check ran. Shared helpers are not treated as coverage of every path.

| Method | Path | Operation ID | SDK method | Status | Tests |
| --- | --- | --- | --- | --- | --- |
| GET | `/discovery` | `discover` | `Service::discover_identifier`, `Client::discover` | Implemented + verified | Version selection, TEA `404` stop, non-TEA `404` failover, `401` no failover, `403` no failover and no token fetch |
| GET | `/products` | `queryProducts` | `Service::query_products` | Implemented + verified | First, next, and repeated opaque `pageToken`; empty page; `pageSize` 101 rejected before the request; paired `idType` and `idValue` are encoded; absent identifier filters are omitted |
| GET | `/product/{uuid}` | `getProduct` | `Service::get_product` | Implemented + verified | `gets_a_product`; `406` is `ClientError::Api` |
| GET | `/product/{uuid}/releases` | `listProductReleases` | `Service::list_product_releases` | Implemented + unverified | Uses `page`. No fixture for this path |
| GET | `/product/{uuid}/cle` | `getProductCle` | `Service::get_product_cle` | Optional; partially verified | `NOT_IMPLEMENTED` is `CleFetch::NotImplemented`. Success and `OBJECT_UNKNOWN` are not fixtures |
| GET | `/productReleases` | `queryProductReleases` | `Service::query_product_releases` | Implemented + unverified | Uses `page` |
| GET | `/productRelease/{uuid}` | `getProductRelease` | `Service::get_product_release` | Implemented + unverified | Uses `get` |
| GET | `/productRelease/{uuid}/cle` | `getProductReleaseCle` | `Service::get_product_release_cle` | Optional; unverified | Same `cle` helper as product CLE |
| GET | `/productRelease/{uuid}/collections` | `listProductReleaseCollections` | `Service::list_product_release_collections` | Implemented + unverified | Uses `page` |
| GET | `/productRelease/{uuid}/collection/latest` | `getLatestProductReleaseCollection` | `Service::get_latest_product_release_collection` | Implemented + unverified | Uses `get` |
| GET | `/productRelease/{uuid}/collection/{collectionVersion}` | `getProductReleaseCollectionByVersion` | `Service::get_product_release_collection` | Implemented + unverified | Uses `get` |
| GET | `/components` | `queryComponents` | `Service::query_components` | Implemented + verified | Malformed page (`hasNext` missing) is `ClientError::Json`. `idType=COMPLIANCE_DOCUMENT` without `idValue` is sent on the first page and kept with `pageToken` on the next page. `idValue` without `idType` returns `ClientError::Model` and sends no request |
| GET | `/component/{uuid}` | `getComponent` | `Service::get_component` | Implemented + partially verified | Missing `Content-Type` (`text/plain`) is `ClientError::Model` |
| GET | `/component/{uuid}/releases` | `listComponentReleases` | `Service::list_component_releases` | Implemented + unverified | Uses `page` |
| GET | `/component/{uuid}/cle` | `getComponentCle` | `Service::get_component_cle` | Optional; unverified | Uses `cle` |
| GET | `/componentReleases` | `queryComponentReleases` | `Service::query_component_releases` | Implemented + unverified | Uses `page` |
| GET | `/componentRelease/{uuid}` | `getComponentRelease` | `Service::get_component_release` | Implemented + unverified | Uses `get` |
| GET | `/componentRelease/{uuid}/cle` | `getComponentReleaseCle` | `Service::get_component_release_cle` | Optional; unverified | Uses `cle` |
| GET | `/componentRelease/{uuid}/collections` | `listComponentReleaseCollections` | `Service::list_component_release_collections` | Implemented + unverified | Uses `page` |
| GET | `/componentRelease/{uuid}/collection/latest` | `getLatestComponentReleaseCollection` | `Service::get_latest_component_release_collection` | Implemented + unverified | Uses `get` |
| GET | `/componentRelease/{uuid}/collection/{collectionVersion}` | `getComponentReleaseCollectionByVersion` | `Service::get_component_release_collection` | Implemented + unverified | Uses `get` |
| GET | `/artifact/{uuid}/latest` | `getLatestArtifact` | `Service::get_latest_artifact` | Implemented + unverified | Uses `get` |
| GET | `/artifact/{uuid}/latest/download` | `downloadLatestArtifact` | `Service::download_latest_artifact` | Implemented + unverified | Uses `download_url` |
| GET | `/artifact/{uuid}/latest/signature/download` | `downloadLatestArtifactSignature` | `Service::download_latest_signature` | Implemented + unverified | Uses `signature_api` |
| GET | `/artifact/{uuid}/{artifactVersion}` | `getArtifactByVersion` | `Service::get_artifact` | Implemented + partially verified | CLI and download fixtures parse artifact JSON. No dedicated error status for this path |
| GET | `/artifact/{uuid}/{artifactVersion}/download` | `downloadArtifactByVersion` | `Service::download_artifact`, `Service::download_format` | Implemented + verified | zlib `deflate` then SHA-256; gzip; checksum mismatch; real bearer stripped off the API prefix; `304` |
| GET | `/artifact/{uuid}/{artifactVersion}/signature/download` | `downloadArtifactSignatureByVersion` | `Service::download_signature` | Implemented + verified | `SIGNATURE_NOT_FOUND`, `OBJECT_UNKNOWN`, present bytes reported `NotDeclared` |
| POST | `/token` | `requestToken` | Internal token exchange | Implemented + verified | One exchange after a bearer `401`. Discovery `403` does not call it |

`HEAD` is described for download operations when a server supports it. `Service::head_artifact` sends `HEAD` and returns headers. It is not a substitute for `GET`.

No other consumer paths are implemented. Publisher operations in `spec/publisher/openapi.json` are out of scope.
