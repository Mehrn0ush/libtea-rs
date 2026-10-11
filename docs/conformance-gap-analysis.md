# Conformance gap analysis

Pinned specification: CycloneDX/transparency-exchange-api `962a0228af1bfcc1232ac3f79239317ff33fdb04`, OpenAPI `3.1.1`, `info.version` `1.0.0`. Retrieved 2026-10-10. Blob of `spec/openapi.yaml`: `c96baa3e102c195895643fab81ae468ee54b8182`.

A row is **Verified** only when a test in this workspace executed the requirement during the 2026-10-10 audit, unless a later note names a newer test. **Code only** means the source implements the rule and no test ran it at that audit. **Incorrect** means the code disagreed with the pinned text at that audit. Server-only obligations are omitted.

Later mock tests and one ignored local Oolong test cover identifier filters. They are named in [api-operation-matrix.md](api-operation-matrix.md) and [interoperability.md](interoperability.md). Where this file's operation table still says "Code only" for a list operation, that is the 2026-10-10 audit. The current filter evidence is REQ-LIST-01 below, not a claim that every list path has its own mock fixture.

The requirements matrix marks several rows as implemented on the strength of code inspection. This document does not treat that as verification.

## Consumer operations

Paths are relative to `{rootUrl}/v{advertised}/`. All 28 operations from `paths` in the pinned OpenAPI file have a method. No extra consumer path is called.

| Operation ID | Method and path | SDK | HTTP test in this audit | Result |
| --- | --- | --- | --- | --- |
| discover | GET `/discovery` | `Client::discover`, `Service::discover_identifier` | TEA 404, non-TEA 404 failover, bearer 401 does not fail over | Partial. PURL query, `400`, and two-release array order are code only |
| queryProducts | GET `/products` | `Service::query_products` | Mock pagination and identifier filters. Ignored live Oolong test | Mock-tested and live-tested for filters. Not a conformance claim. The 2026-10-10 audit had no HTTP test for this path |
| getProduct | GET `/product/{uuid}` | `Service::get_product` | `gets_a_product` | Verified for one JSON object |
| listProductReleases | GET `/product/{uuid}/releases` | `Service::list_product_releases` | None | Code only |
| getProductCle | GET `/product/{uuid}/cle` | `Service::get_product_cle` | None | Code only. `NOT_IMPLEMENTED` branch is code only |
| queryProductReleases | GET `/productReleases` | `Service::query_product_releases` | Ignored live Oolong type-only filter. No mock for this path | Live-tested for one PURL filter. The 2026-10-10 audit was code only |
| getProductRelease | GET `/productRelease/{uuid}` | `Service::get_product_release` | None | Code only |
| getProductReleaseCle | GET `/productRelease/{uuid}/cle` | `Service::get_product_release_cle` | None | Code only |
| listProductReleaseCollections | GET `/productRelease/{uuid}/collections` | `Service::list_product_release_collections` | None | Code only |
| getLatestProductReleaseCollection | GET `/productRelease/{uuid}/collection/latest` | `Service::get_latest_product_release_collection` | None | Code only |
| getProductReleaseCollectionByVersion | GET `/productRelease/{uuid}/collection/{collectionVersion}` | `Service::get_product_release_collection` | None | Code only |
| queryComponents | GET `/components` | `Service::query_components` | Mock malformed page and identifier filters. Ignored live Oolong type-only filter | Mock-tested and live-tested for filters. The 2026-10-10 audit was code only |
| getComponent | GET `/component/{uuid}` | `Service::get_component` | None | Code only |
| listComponentReleases | GET `/component/{uuid}/releases` | `Service::list_component_releases` | None | Code only |
| getComponentCle | GET `/component/{uuid}/cle` | `Service::get_component_cle` | None | Code only |
| queryComponentReleases | GET `/componentReleases` | `Service::query_component_releases` | Ignored live Oolong type-only filter. No mock for this path | Live-tested for one PURL filter. The 2026-10-10 audit was code only |
| getComponentRelease | GET `/componentRelease/{uuid}` | `Service::get_component_release` | None | Code only |
| getComponentReleaseCle | GET `/componentRelease/{uuid}/cle` | `Service::get_component_release_cle` | None | Code only |
| listComponentReleaseCollections | GET `/componentRelease/{uuid}/collections` | `Service::list_component_release_collections` | None | Code only |
| getLatestComponentReleaseCollection | GET `/componentRelease/{uuid}/collection/latest` | `Service::get_latest_component_release_collection` | None | Code only |
| getComponentReleaseCollectionByVersion | GET `/componentRelease/{uuid}/collection/{collectionVersion}` | `Service::get_component_release_collection` | None | Code only |
| getLatestArtifact | GET `/artifact/{uuid}/latest` | `Service::get_latest_artifact` | None | Code only |
| downloadLatestArtifact | GET `/artifact/{uuid}/latest/download` | `Service::download_latest_artifact` | None | Code only |
| downloadLatestArtifactSignature | GET `/artifact/{uuid}/latest/signature/download` | `Service::download_latest_signature` | None | Code only |
| getArtifactByVersion | GET `/artifact/{uuid}/{artifactVersion}` | `Service::get_artifact` | None | Code only |
| downloadArtifactByVersion | GET `/artifact/{uuid}/{artifactVersion}/download` | `Service::download_artifact`, `Service::download_format` | One GET of a versioned download, no content coding | Partial |
| downloadArtifactSignatureByVersion | GET `/artifact/{uuid}/{artifactVersion}/signature/download` | `Service::download_signature` | None | Code only. `SIGNATURE_NOT_FOUND` is code only |
| requestToken | POST `/token` | `ClientInner::fetch_token` | `bearer_challenge_exchanges_an_api_key_once` expects one POST | Partial. Basic encoding of a colon in the secret is code only |

`406` is a documented response on the four download operations and is mapped to `ClientError::Api`. `not_acceptable_is_an_api_error_and_a_missing_json_type_is_rejected` sends `406` on `GET /product/{uuid}` and rejects a JSON endpoint whose `Content-Type` is `text/plain`. The client does not invent an `Accept` media type. `415` is not a response in the pinned consumer paths that were extracted.

## Normative client requirements

| ID | Source | Requirement | Implementation | Test | Result |
| --- | --- | --- | --- | --- | --- |
| REQ-TEI-01 | discovery/readme.md, TEI syntax | Form `tei://domain/type/id`. Type is lowercase ASCII starting with a letter. No port. Identifier is one segment and rejects `/` and `%2F` | `tea_core::Tei::parse` | `rejects_slash_and_port`, `parses_registry_examples` | Verified |
| REQ-TEI-02 | Comparing TEIs | Equality is the original character sequence. Clients shall not rewrite a TEI | `Tei` stores `original` and displays it | `does_not_treat_case_variants_as_equal`, property test | Verified |
| REQ-TEI-03 | TEI type registry examples in the discovery document | Registered examples parse. Unknown types stay valid | `Tei::validate_registry` | `parses_registry_examples`, `unknown_type_parses_without_registry_rules` | Verified for the examples in the discovery table. The registry files themselves were not re-diffed byte for byte in this audit |
| REQ-VER-01 | Selecting an API base | Highest mutually supported SemVer, build metadata ignored for comparison and kept in the path, `+` not encoded | `select_endpoints`, `versioned_api_prefix` | `prefers_higher_version_over_priority`, `keeps_build_metadata_spelling`, `prerelease_is_not_the_stable_version` | Verified |
| REQ-VER-02 | Same section | Omitted priority is 1. Range 0 to 1 | `OrderedPriority` | `omitted_priority_is_one_and_http_base_parses` | Verified |
| REQ-VER-03 | Failover section | Next attempt should follow priority order | Sort is version, then priority, then document index | Version-over-priority unit test | Ambiguity. The selection section requires version first. The failover sentence says priority order. Code follows the selection section |
| REQ-DISC-01 | Discovery response | Success is a non-empty array. Empty `200` is not success | `DiscoveryInfo::parse_list` | `empty_discovery_array_is_rejected` | Verified |
| REQ-DISC-02 | Unresolved identifiers | TEA `404` (`application/json` object with string `error`) stops discovery, including an unknown code and extra fields | `tea_error_code`, `Client::discover` | `discovery_follows_version_and_stops_on_tea_404`, unit recognition of `FUTURE` | Verified |
| REQ-DISC-03 | Failover | Non-TEA `404` tries the next compatible endpoint | `Client::discover` | `non_tea_404_fails_over_to_the_next_endpoint` | Verified |
| REQ-DISC-04 | Authentication section | `401` and `403` do not fail over | `Client::discover` returns on those statuses and on `AuthenticationRequired` | `bearer_401_does_not_fail_over`, `discovery_403_does_not_fail_over_or_fetch_a_token` | Mock-tested. The 2026-10-10 audit had verified `401` only |
| REQ-DISC-05 | Discovery by PURL | Exactly one of `tei` and `purl` | `Service::discover_identifier` | None | Code only |
| REQ-DISC-06 | Discovery response | `servers[]` selected with the same version and priority rules. Further calls use that root, not the well-known host, unless it is also listed | `DiscoveryInfo::select_server`, then only the first selected server is kept | None for `servers[]` | Code only. Alternate servers are dropped after selection |
| REQ-DISC-07 | Query examples | Discovery prose example leaves `/` unencoded. OpenAPI example percent-encodes the whole TEI | `operation_url` uses `query_pairs_mut`, which encodes `/` | `query_encoding_matches_openapi_example_shape` | Ambiguity. Client matches the OpenAPI example. Not verified against a server |
| REQ-AUTH-01 | auth/readme.md | API key is Basic on `POST /token` after form-encoding each part. Body is `grant_type=client_credentials` | `basic_header`, `fetch_token` | Token POST is counted once. The test does not decode the Basic header | Partial |
| REQ-AUTH-02 | Using the access token | Resource calls use `Authorization: Bearer`. The token is not parsed | `bearer_for` | Bearer retry test | Verified for one product GET |
| REQ-AUTH-03 | RFC 6750 as cited | A bearer challenge with no error, or `invalid_token`, may refresh once | `should_refresh_token` | One successful exchange | Partial. `invalid_token` and a second `401` are code only |
| REQ-AUTH-04 | Discovery authentication | Do not probe `/token` before a resource asks | Token fetch runs only from a `401` handler | No startup call in the client constructor | Code only |
| REQ-AUTH-05 | Same section | Do not send the access token to another origin or outside the API prefix. Do not send it to an external artifact URL | `url_under_prefix`, `download_url(..., under_api: false)` | `bearer_token_is_kept_under_the_prefix_and_stripped_after_that`, `redirect_to_another_origin_is_refused_before_a_credential_is_sent` | Verified. The earlier fixture had no API key (TEA-AUD-004). The replacement installs a real token and fails if `Authorization` reaches the outside path |
| REQ-AUTH-06 | TLS | Verify certificates. Do not disable verification | rustls, `min_tls_version` TLS 1.2, no invalid-cert option | None against a bad certificate | Code only |
| REQ-ART-01 | tea-collection.md | Checksums cover bytes after content coding is removed. A mismatch is not the revision | `verify_checksums`, `decode_content_coding` | SHA-256 mismatch unit test. HTTP test uses an uncompressed body | Partial |
| REQ-ART-02 | Same, and RFC 9110 `deflate` | Remove HTTP content coding. `deflate` is the zlib format | `flate2::write::ZlibDecoder` inside `ContentDecoder` | `http_deflate_is_zlib_and_raw_deflate_is_rejected`, `zlib_deflate_is_decoded_before_the_checksum`, gzip HTTP and unit tests | Verified. Was incorrect (TEA-AUD-001): the old code used raw `DeflateDecoder` |
| REQ-ART-03 | OpenAPI download | Strong `ETag` is a validator, not a checksum. `304` returns no body. Weak tags are not strong identity | `DownloadedArtifact::strong_etag`, `ArtifactFetch::NotModified` | `conditional_get_304_is_not_a_checksum_and_has_no_body`; weak `ETag` flag in the uncompressed download test | Verified. `304` was previously code only |
| REQ-ART-04 | OpenAPI | Servers that support `HEAD` return the same headers as `GET` and no body | `Service::head_artifact`; `HEAD` and `304` skip the body budget | `head_ignores_representation_content_length`, `head_without_content_length_returns_metadata`, `head_redirect_uses_the_final_headers_and_strips_a_real_token` | Verified. Was incorrect (TEA-AUD-002) |
| REQ-ART-05 | Collection | A format with `url` has checksums. External URL is fetched without a bearer token | `ArtifactFormat::validate`, `download_format` | `artifact_with_url_requires_checksum` (parse only) | Parse rule verified. External GET is code only |
| REQ-ART-06 | Collection | `latest` download is mutable and is not a format `url` | Client exposes `download_latest_artifact` and does not write `latest` into a format URL | None | Code only |
| REQ-PAGE-01 | OpenAPI pagination | `pageToken` is copied from `nextPageToken`. `pageSize` is bounded | `ListParams::validate` (1..=100) and `page` | `product_pages_keep_tokens_opaque_and_do_not_follow_urls`, `empty_page_and_malformed_page_and_oversized_page_size`, `type_only_filter_stays_on_the_next_page` | Verified for product and component list calls. The client does not follow a next-page URL. A token that looks like `https://169.254.169.254/latest` stays a query value on the API origin. A type-only `idType` is repeated with the continuation token and is not replaced by `idValue`. Collection list endpoints use the same `page` helper and do not have their own fixtures |
| REQ-LIST-01 | OpenAPI `idType` / `idValue`, compliance-document discovery | `idType` alone is a filter this client can send. Both parameters are sent when both are set | `ListParams::validate`, `Service::page` | Mock: `type_only_identifier_filter_is_serialized`, `both_identifier_filters_are_encoded_in_the_query`, `absent_identifier_filters_are_omitted`, `value_only_identifier_filter_sends_no_request`. Live: ignored `oolong_idtype_live` | Mock-tested and live-tested against one local Oolong. `idValue` without `idType` fails before HTTP. The error text still says the fields must be provided together, which overstates the check. The pinned specification does not settle the value-only case, and type-only support here is not a rule for every server |
| REQ-HTTP-01 | Discovery TLS and this SDK's destination policy | Default client refuses cleartext, loopback, and link-local literals | `DestinationPolicy::check_url` | `production_policy_rejects_cleartext_and_metadata` | Verified for those literals. DNS filtering was not executed against a resolver |
| REQ-CACHE-01 | Discovery data caching | Implementations should respect caching headers and refresh | No cache. Callers may pass `If-None-Match` on downloads | None | Not a mandatory "shall". Not implemented |

## Specification ambiguities

These are not counted as client defects.

| Topic | Pinned text | Client choice |
| --- | --- | --- |
| README status | README says Beta 2. OpenAPI `info.version` is `1.0.0` | OpenAPI `1.0.0` |
| `error-response` | Schema sets `additionalProperties: false` and a closed error enum. Discovery prose says unknown `error` strings and unrecognized properties must not become failover | Recognition follows the prose |
| Failover order | Selection is version, then priority. Failover says preserve priority order | Version, then priority, then document index |
| `tei` query encoding | Discovery examples encode only `:`. The OpenAPI description's example encodes `/` as well | Full query encoding of non-unreserved bytes |
| Collection and artifact `version` | Prose starts at 1. Schema type is integer with no `minimum` | `validate` rejects values below 1 |
| Well-known `http` | Schema pattern allows `http`. Prose says a client shall reject it unless that base was configured for local testing | Documents parse. The default policy refuses the connection |
| Signatures technology | `signatures/signature.md` is outside the standard and mentions `/trust-anchors/`, which is not in this OpenAPI | Signature bytes can be downloaded. Trust anchors are not implemented |
| Value-only `idValue` | Compliance-document prose shows `idType` without `idValue`. It does not define `idValue` without `idType` | This client rejects the value-only combination before HTTP. That is SDK behavior, not a new specification rule |

## What the static conformance report covers

`tea_conformance::static_report` checks six local cases: registry TEI examples, case-sensitive equality, the well-known priority example, an empty discovery array, an unknown error code, and a wrong SHA-256. It does not open a socket. A passing report is not a conformance certificate.
