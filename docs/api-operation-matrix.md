# Consumer API operations

Source: vendored OpenAPI at `spec/vendor/openapi.yaml` (commit
`962a0228af1bfcc1232ac3f79239317ff33fdb04`, `info.version` `1.0.0`). See
`spec/PINNED.md`.

Paths below are relative to `{rootUrl}/v{advertised-version}`. Every operation
except `POST /token` accepts an optional bearer token. `/token` uses HTTP Basic
for the API-key baseline. This SDK implements the Basic baseline.

Global error responses `400`, `401`, and `403` apply where the operation lists
them. `404` uses the shared TEA error body. Clients do not treat status `404`
alone as "does not exist".

## Evidence labels

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

### Cross-cutting negatives (not a single operation)

| Claim | Tested |
| --- | --- |
| Oversized JSON body rejected | `oversized_json_is_rejected` |
| Unknown enum / error strings preserved | `preserves_unknown_artifact_type` (core); `unknown_error_is_still_a_tea_error` (conformance) |
| Pagination token round-trip (opaque) | `product_pages_keep_tokens_opaque_and_do_not_follow_urls`; `type_only_filter_stays_on_the_next_page` |
| Value-only identifier filter rejected pre-HTTP | `value_only_identifier_filter_sends_no_request` |
| `follow_pages` limits / loop guard | `follow_pages_multi_page_then_exhausted`; `follow_pages_empty_page_exhausts`; `follow_pages_hits_max_items`; `follow_pages_repeated_token_guard`; unit `stops_on_repeated_token`; `respects_max_pages`; `empty_next_token_is_exhausted_not_followed` |

No other consumer paths are implemented. Publisher operations are out of scope.

### Remaining caveats (still true)

- Signature download tests assert bytes / absence only — not authenticity.
- Oolong CLE and `/token` live results are server limitations, not SDK defects.
- Vulnetix evidence is for that named deployment only.
