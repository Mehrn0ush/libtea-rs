# Operation matrix route audit

Method: for each Implemented / Optional row in `docs/api-operation-matrix.md`,
open the cited mock test and record the wiremock `method` + `path` matcher and
whether a success status (2xx, or intentional CLE/signature outcome) is asserted.

Directory paths are under `crates/tea-client/tests/`. Package name: `libtea-client`.

| Operation | Expected route | Primary test | Matcher path found | Verdict |
| --- | --- | --- | --- | --- |
| discover | GET `/discovery` | `discovery_follows_version_and_stops_on_tea_404` | GET `/v1.0.0/discovery` (200/404 paths) | EXACT |
| queryProducts | GET `/products` | `product_pages_keep_tokens_opaque_and_do_not_follow_urls` | GET `/v1.0.0/products` 200 | EXACT |
| getProduct | GET `/product/{uuid}` | `gets_a_product` | GET `/v1.0.0/product/{uuid}` 200 | EXACT |
| listProductReleases | GET `/product/{uuid}/releases` | `list_and_query_product_releases_paths_and_params` | GET `/v1.0.0/product/{uuid}/releases` 200 | EXACT |
| getProductCle | GET `/product/{uuid}/cle` | `cle_success_for_every_cle_endpoint` | GET `/v1.0.0/product/{uuid}/cle` 200 | EXACT |
| queryProductReleases | GET `/productReleases` | `list_and_query_product_releases_paths_and_params` | GET `/v1.0.0/productReleases` 200 | EXACT |
| getProductRelease | GET `/productRelease/{uuid}` | `product_release_get_and_unknown_identifier` | GET `/v1.0.0/productRelease/{uuid}` 200 | EXACT |
| getProductReleaseCle | GET `/productRelease/{uuid}/cle` | `cle_success_for_every_cle_endpoint` | GET `/v1.0.0/productRelease/{uuid}/cle` 200 | EXACT |
| listProductReleaseCollections | GET `/productRelease/{uuid}/collections` | `pagination_anomalies_are_visible_to_callers` (`consumer_surface.rs:478`) | GET `/v1.0.0/productRelease/{uuid}/collections` + `pageSize=2`, `sortField=version`, `pageToken` missing / `=mid-empty`; 200 | EXACT |
| getLatestProductReleaseCollection | GET `…/collection/latest` | `product_release_collection_latest_and_by_version` | GET `/v1.0.0/productRelease/{uuid}/collection/latest` 200 | EXACT |
| getProductReleaseCollectionByVersion | GET `…/collection/{v}` | `product_release_collection_latest_and_by_version` | GET `/v1.0.0/productRelease/{uuid}/collection/2` 200 | EXACT |
| queryComponents | GET `/components` | `type_only_filter_stays_on_the_next_page` (`protocol.rs:906`) | GET `/v1.0.0/components` + `idType=COMPLIANCE_DOCUMENT`, `pageSize=1`, `idValue` missing, `pageToken` missing / `=page-2`; 200 | EXACT |
| getComponent | GET `/component/{uuid}` | `get_component_happy_path` | GET `/v1.0.0/component/{uuid}` 200 | EXACT |
| listComponentReleases | GET `/component/{uuid}/releases` | `list_and_query_component_releases_paths` | GET `/v1.0.0/component/{uuid}/releases` 200 | EXACT |
| getComponentCle | GET `/component/{uuid}/cle` | `cle_success_for_every_cle_endpoint` | GET `/v1.0.0/component/{uuid}/cle` 200 | EXACT |
| queryComponentReleases | GET `/componentReleases` | `list_and_query_component_releases_paths` | GET `/v1.0.0/componentReleases` 200 | EXACT |
| getComponentRelease | GET `/componentRelease/{uuid}` | `component_release_collections_and_versioned_gets` | GET `/v1.0.0/componentRelease/{uuid}` 200 | EXACT |
| getComponentReleaseCle | GET `/componentRelease/{uuid}/cle` | `cle_success_for_every_cle_endpoint` | GET `/v1.0.0/componentRelease/{uuid}/cle` 200 | EXACT |
| listComponentReleaseCollections | GET `…/collections` | `component_release_collections_and_versioned_gets` | GET `/v1.0.0/componentRelease/{uuid}/collections` 200 | EXACT |
| getLatestComponentReleaseCollection | GET `…/collection/latest` | `component_release_collection_latest_and_by_version` | GET `/v1.0.0/componentRelease/{uuid}/collection/latest` 200 | EXACT |
| getComponentReleaseCollectionByVersion | GET `…/collection/{v}` | `component_release_collection_latest_and_by_version` | GET `/v1.0.0/componentRelease/{uuid}/collection/1` 200 | EXACT |
| getLatestArtifact | GET `/artifact/{uuid}/latest` | `artifact_latest_versus_revision_and_media_type` | GET `/v1.0.0/artifact/{uuid}/latest` 200 | EXACT |
| downloadLatestArtifact | GET `…/latest/download` | `download_latest_artifact_and_signature_aliases` | GET `/v1.0.0/artifact/{uuid}/latest/download` 200 | EXACT |
| downloadLatestArtifactSignature | GET `…/latest/signature/download` | `download_latest_artifact_and_signature_aliases` | GET `/v1.0.0/artifact/{uuid}/latest/signature/download` 200 | EXACT (bytes only; not crypto) |
| getArtifactByVersion | GET `/artifact/{uuid}/{v}` | `artifact_latest_versus_revision_and_media_type` | GET `/v1.0.0/artifact/{uuid}/1` 200 | EXACT |
| downloadArtifactByVersion | GET `/artifact/{uuid}/{v}/download` | `zlib_deflate_is_decoded_before_the_checksum` (`protocol.rs:380`) | GET `/v1.0.0/artifact/{uuid}/1/download` 200 (`content-encoding: deflate`); no `mediaType` matcher | EXACT |
| downloadArtifactSignatureByVersion | GET `…/{v}/signature/download` | `signature_bytes_are_not_reported_as_verified` | GET `/v1.0.0/artifact/{uuid}/1/signature/download` 200 | EXACT (bytes only; not crypto) |
| requestToken | POST `/token` | `client_credentials_uses_basic_and_a_form_body` | POST `/v1.0.0/token` 200 | EXACT |
| head_artifact | HEAD download paths | `head_ignores_representation_content_length` | HEAD `/v1.0.0/artifact/{uuid}/1/download` 200 | EXACT |

## Corrections made during this audit

| Issue | Fix |
| --- | --- |
| `getComponent` previously cited only `not_acceptable_…` which hits `/component/{uuid}` **without** `Content-Type` (error path), not a 200 body | Added `get_component_happy_path` (200 JSON); matrix updated to cite it |
| `cle_success_and_object_unknown` for productRelease CLE is **404 OBJECT_UNKNOWN**, not CLE success | Matrix already also cites `cle_success_for_every_cle_endpoint` for success; keep both |
| (2026-11-07) downloadArtifactByVersion row cited `artifact_checksum_covers_decoded_bytes_…` with a truncated name and recorded "…/1/download 200". That test's download-path matcher answers **302**; the 200 comes from `/files/sbom` after the redirect. | Primary test is now `zlib_deflate_is_decoded_before_the_checksum` (exact path, 200). The redirect test is kept as redirect / credential-stripping evidence: `artifact_checksum_covers_decoded_bytes_and_not_a_redirect_with_credentials` (`http.rs:245`). |
| (2026-11-07) getComponentCle cited `cle_success_and_object_unknown`; for `/component/{uuid}/cle` that test answers **404 NOT_IMPLEMENTED**, not success | Annotated in the matrix; success evidence is `cle_success_for_every_cle_endpoint` |

## Neighbour citations removed / clarified

- `oversized_json_is_rejected` remains a cross-cutting negative for getProduct size limits (not the primary success evidence).
- `not_acceptable_…` remains useful for content-type/error behaviour on `/component/{uuid}` but is no longer the sole Tested entry for getComponent success.

## Independent re-verification (2026-11-07 audit)

These rows were re-checked by opening each matcher, not by trusting the table
above. Paths are under `crates/tea-client/tests/`.

| Operation | Test (file:line) | Method | Path matcher | Query / header matchers | Status asserted | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| downloadArtifactByVersion | `zlib_deflate_is_decoded_before_the_checksum` (`protocol.rs:380`) | GET | `/v1.0.0/artifact/{ID}/1/download` | none (no `mediaType`) | 200 | EXACT (row corrected) |
| downloadArtifactByVersion (redirect) | `artifact_checksum_covers_decoded_bytes_and_not_a_redirect_with_credentials` (`http.rs:245`) | GET | `/v1.0.0/artifact/{ID}/1/download` | none | **302** → `/files/sbom` 200; `authorization` present → 500 | Redirect evidence, not a 200 on the API path |
| listProductReleaseCollections | `pagination_anomalies_are_visible_to_callers` (`consumer_surface.rs:478`) | GET | `/v1.0.0/productRelease/{ID}/collections` | `pageSize=2`, `sortField=version`, `pageToken` missing / `mid-empty` | 200 | EXACT |
| queryComponents | `type_only_filter_stays_on_the_next_page` (`protocol.rs:906`) | GET | `/v1.0.0/components` | `idType`, `pageSize=1`, `idValue` missing, `pageToken` | 200 | EXACT |
| getLatestProductReleaseCollection | `product_release_collection_latest_and_by_version` (`coverage_gaps.rs:85`) | GET | `/v1.0.0/productRelease/{ID}/collection/latest` | none | 200, `expect(1)` | EXACT |
| getProductReleaseCollectionByVersion | same (`coverage_gaps.rs:85`) | GET | `/v1.0.0/productRelease/{ID}/collection/2` | none | 200, `expect(1)` | EXACT |
| getLatestComponentReleaseCollection | `component_release_collection_latest_and_by_version` (`coverage_gaps.rs:114`) | GET | `/v1.0.0/componentRelease/{ID}/collection/latest` | none | 200, `expect(1)` | EXACT |
| getComponentReleaseCollectionByVersion | same (`coverage_gaps.rs:114`) | GET | `/v1.0.0/componentRelease/{ID}/collection/1` | none | 200, `expect(1)` | EXACT |
| getProductCle | `cle_success_for_every_cle_endpoint` (`coverage_gaps.rs:206`) | GET | `/v1.0.0/product/{ID}/cle` | none | 200, `expect(1)`, `CleFetch::Events` | EXACT |
| getProductReleaseCle | same | GET | `/v1.0.0/productRelease/{ID}/cle` | none | 200, `expect(1)` | EXACT |
| getComponentCle | same | GET | `/v1.0.0/component/{ID}/cle` | none | 200, `expect(1)` | EXACT |
| getComponentReleaseCle | same | GET | `/v1.0.0/componentRelease/{ID}/cle` | none | 200, `expect(1)` | EXACT |

Not re-verified in this pass: the Oolong and Vulnetix columns of the matrix
(live evidence; no access under the audit restrictions).
