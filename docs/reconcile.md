# Documentation reconciliation: cited names vs code

Generated on 2026-11-08 against the working tree on top of `46927402c65c0f43d6952343f6be0506eade36f2` (CHECKPOINT; nothing newer is committed). Method: every backticked token in `docs/*.md`, `docs/milestones/*.md`, `REPORT.md` and `AUDIT.md` that looks like a function/test name (`snake_case`, `name()`), a path (`A::b`), or a CamelCase type was looked up in `crates/**/*.rs`, `fuzz/fuzz_targets/*.rs` and `scripts/*.sh` (as `fn`/type definition, any identifier, or a test/target file name). Protocol words such as `access_token` and `invalid_token` are skipped. A few names are classified by hand (listed in "Not code / historical").

| Status | Count |
| --- | --- |
| absent by design | 1 |
| exists | 434 |
| external | 22 |
| not code | 6 |
| planned (not code yet) | 2 |
| removed (historical) | 6 |
| serde attribute | 1 |
| **total** | 472 |

**No cited test is missing.** Every test name in the matrix, audit, and auth documents resolves to a `fn` in `crates/` (the earlier mechanical check of `docs/api-operation-matrix.md` and `docs/matrix-audit.md` is repeated here as part of the table).

## Not code, removed, or needing attention

| Cited name | Status | Note | Cited in |
| --- | --- | --- | --- |
| `DeflateDecoder` | removed (historical) | code before TEA-AUD-001; cited as the finding | conformance-gap-analysis.md, security-review.md |
| `DestinationPolicy::host_excepted` | removed (historical) | removed in the 2026-10-11 remediation; cited as the finding | security-review.md |
| `DestinationPolicy::only_hosts` | planned (not code yet) | proposed name for work item 0.4-4; the exact API is decided when that item starts | milestones/0.4.md |
| `Limits::max_retry_after` | planned (not code yet) | proposed name for work item 0.2-9; the exact API is decided when that item starts | milestones/0.2.md |
| `ValidAndTrusted` | not code | proposed outcome name in docs/design/signature-verification.md; no such variant exists (by design, 0.6 work) | roadmap-gap-analysis.md |
| `ValidButUntrusted` | not code | proposed outcome name in docs/design/signature-verification.md; no such variant exists (by design, 0.6 work) | roadmap-gap-analysis.md |
| `danger_accept_invalid_certs` | absent by design | reqwest API that this repository must not call; docs cite its absence; `grep` over crates/ finds none | claims-audit.md |
| `deny_unknown_fields` | serde attribute | removed from discovery types on 2026-11-08; remaining mentions are historical or describe the removal | AUDIT.md, REPORT.md, roadmap-gap-analysis.md |
| `discard_token` | removed (historical) | replaced by `discard_token_if_current`; cited as the finding | security-review.md |
| `flate2::read::DeflateDecoder` | removed (historical) | code before TEA-AUD-001; cited as the finding | security-review.md |
| `flate2::write::ZlibDecoder` | removed (historical) | replaced by `ZlibInflate`; present-tense mentions fixed on 2026-11-08, the rest describe the pre-fix state | AUDIT.md, security-review.md |
| `host_excepted` | removed (historical) | removed in the 2026-10-11 remediation; cited as the finding | security-review.md |
| `no_run` | not code | rustdoc code-block attribute | roadmap-gap-analysis.md |
| `no_std` | not code | Rust attribute named as a non-goal | roadmap.md, specification-baseline.md |
| `no_such_option` | not code | deliberately nonexistent method used to show that a broken README snippet fails | AUDIT.md |
| `pull_request_target` | not code | GitHub Actions trigger named as absent from the workflows | AUDIT.md |

## Stale references fixed in this pass

| Where | Was | Now |
| --- | --- | --- |
| `docs/conformance-gap-analysis.md` REQ-ART-02 | `flate2::write::ZlibDecoder` inside `ContentDecoder` | `ZlibInflate` (wrapper over `flate2::Decompress`) |
| `docs/security-review.md` (status after remediation) | "`deflate` uses `flate2::write::ZlibDecoder`" | `ZlibInflate`, noting what it replaced |
| `AUDIT.md` "Areas reviewed" | "`deny_unknown_fields` on discovery wire types matches `additionalProperties: false`" | states the attribute was removed on 2026-11-08 and why |
| `docs/roadmap-gap-analysis.md` | "unknown fields rejected", "urn: not a clear message", "`swid` Missing", exhaustive models, "README snippets not compiled" | updated for the 2026-11-08 changes; `swid` recorded as not a registered TEI type |
| `docs/auth.md`, `docs/security.md`, `README.md`, `docs/interoperability.md` | "client-wide mutex" wording | fixed in audit round 1 (per-origin locks) |
| `REPORT.md`, `AUDIT.md` | test counts of 147 / 159 / 198 mixed without labels | labelled per tree: as found 147, round 1 159, audited tree 198 (188 passed + 10 ignored), after pre-0.1.0 changes 219 (209 passed + 10 ignored) |

Historical documents (`docs/audit-report.md`, `docs/security-review.md`, `docs/conformance-gap-analysis.md`) deliberately keep names of code that has since been removed; those rows are marked "removed (historical)" above.

## All cited names

| Cited name | Status | Cited in |
| --- | --- | --- |
| `ApiKey` | exists | auth.md, roadmap-gap-analysis.md, security-review.md, security.md |
| `ApiVersion` | exists | roadmap-gap-analysis.md |
| `ApiVersion::parse` | exists | interoperability-plan.md |
| `ApiVersion::parse_lenient` | exists | AUDIT.md, REPORT.md, roadmap-gap-analysis.md |
| `ApiVersion::semver()` | exists | roadmap-gap-analysis.md |
| `ArtifactFetch` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `ArtifactFetch::NotModified` | exists | conformance-gap-analysis.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `ArtifactFormat` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `ArtifactFormat::validate` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `ArtifactType` | exists | milestones/0.3.md, roadmap-gap-analysis.md |
| `AuthenticationRequired` | exists | conformance-gap-analysis.md |
| `CappedBuf` | exists | roadmap-gap-analysis.md |
| `Checksum::new` | exists | AUDIT.md, REPORT.md |
| `ChecksumAlgorithm` | exists | roadmap-gap-analysis.md |
| `ChecksumAlgorithm::Other` | exists | architecture.md, roadmap-gap-analysis.md |
| `ChecksumAlgorithm::is_weak` | exists | AUDIT.md, REPORT.md |
| `ChecksumCheck` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `ChecksumCheck::weak` | exists | AUDIT.md, REPORT.md, security.md |
| `ChecksumPolicy` | exists | AUDIT.md, REPORT.md, roadmap-gap-analysis.md |
| `ChecksumPolicy::with_allow_weak` | exists | security.md |
| `ChecksumVerifier` | exists | milestones/0.3.md, roadmap-gap-analysis.md |
| `ChecksumVerifier::update` | exists | security-review.md |
| `ChecksumVerifier::with_policy` | exists | REPORT.md |
| `CleDefinitions` | exists | roadmap-gap-analysis.md |
| `CleEvent` | exists | roadmap-gap-analysis.md |
| `CleFetch` | exists | AUDIT.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `CleFetch::Events` | exists | matrix-audit.md, roadmap-gap-analysis.md |
| `CleFetch::NotImplemented` | exists | interoperability.md |
| `CleSupportDefinition` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `CleVersionSpecifier` | exists | roadmap-gap-analysis.md |
| `Client::builder()` | exists | release-checklist.md |
| `Client::discover` | exists | AUDIT.md, api-operation-matrix.md, architecture.md, conformance-gap-analysis.md, interoperability-plan.md, interoperability.md, roadmap-gap-analysis.md, security.md |
| `Client::service` | exists | security.md |
| `ClientBuilder` | exists | roadmap-gap-analysis.md |
| `ClientBuilder::allow_discovered_credentials` | exists | requirements-matrix.md |
| `ClientBuilder::build` | exists | AUDIT.md, roadmap-gap-analysis.md, security-review.md |
| `ClientBuilder::lenient_versions` | exists | AUDIT.md, REPORT.md, roadmap-gap-analysis.md |
| `ClientBuilder::well_known_fetch_url` | exists | roadmap-gap-analysis.md, security-review.md |
| `ClientError` | exists | AUDIT.md, auth.md, open-questions.md, roadmap-gap-analysis.md |
| `ClientError::Api` | exists | AUDIT.md, conformance-gap-analysis.md, open-questions.md, roadmap-gap-analysis.md |
| `ClientError::Integrity` | exists | claims-audit.md, security.md |
| `ClientError::Json` | exists | roadmap-gap-analysis.md |
| `ClientError::Policy` | exists | open-questions.md, security-review.md |
| `ClientError::Timeout` | exists | auth.md |
| `ClientError::Token` | exists | auth.md |
| `ClientError::TooLarge` | exists | security-review.md |
| `ClientError::Transport` | exists | release-readiness.md, security.md |
| `ClientError::Unresolved` | exists | interoperability-plan.md |
| `ClientError::transport` | exists | security.md |
| `ClientInner::fetch_token` | exists | conformance-gap-analysis.md |
| `CollectionSort` | exists | roadmap-gap-analysis.md |
| `CollectionUpdateReason` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `ComplianceDocument` | exists | milestones/0.2.md |
| `ComponentRef` | exists | AUDIT.md |
| `ComponentRelease` | exists | roadmap-gap-analysis.md |
| `ComponentSort` | exists | roadmap-gap-analysis.md |
| `ContentDecoder` | exists | conformance-gap-analysis.md, release-readiness.md, requirements-matrix.md, roadmap-gap-analysis.md, security-review.md |
| `DeflateDecoder` | removed (historical) | conformance-gap-analysis.md, security-review.md |
| `DestinationPolicy` | exists | AUDIT.md, milestones/0.4.md, requirements-matrix.md, roadmap-gap-analysis.md, security.md |
| `DestinationPolicy::allow_origin` | exists | interoperability-plan.md, security-review.md, security.md, specification-ambiguities.md |
| `DestinationPolicy::check_url` | exists | conformance-gap-analysis.md, roadmap-gap-analysis.md |
| `DestinationPolicy::host_excepted` | removed (historical) | security-review.md |
| `DestinationPolicy::only_hosts` | planned (not code yet) | milestones/0.4.md |
| `DiscoveryFailed` | exists | roadmap-gap-analysis.md |
| `DiscoveryInfo` | exists | REPORT.md, roadmap-gap-analysis.md |
| `DiscoveryInfo::parse_list` | exists | conformance-gap-analysis.md |
| `DiscoveryInfo::parse_list_lenient` | exists | REPORT.md |
| `DiscoveryInfo::select_server` | exists | conformance-gap-analysis.md |
| `DiscoveryServer` | exists | REPORT.md, roadmap-gap-analysis.md |
| `DownloadedArtifact` | exists | roadmap-gap-analysis.md |
| `DownloadedArtifact::integrity` | exists | claims-audit.md |
| `DownloadedArtifact::strong_etag` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `EndOfLife` | exists | milestones/0.5.md |
| `EndOfSupport` | exists | milestones/0.5.md |
| `EnumKnowledge` | exists | roadmap-gap-analysis.md |
| `FollowedPage` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `GuardSortOrder` | exists | AUDIT.md |
| `GzDecoder` | exists | security-review.md |
| `IntegrityOutcome` | exists | AUDIT.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `IntegrityOutcome::NotDeclared` | exists | security-review.md |
| `IntegrityOutcome::Unsupported` | exists | architecture.md |
| `IntegrityOutcome::Verified` | exists | AUDIT.md, audit-report.md, claims-audit.md, interoperability-plan.md, interoperability.md, security-review.md, security.md |
| `IntegrityOutcome::Weak` | exists | AUDIT.md, REPORT.md, open-questions.md, security.md |
| `IntegrityReport` | exists | roadmap-gap-analysis.md |
| `IpClass` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `Limits::connect_timeout` | exists | auth.md |
| `Limits::max_discovery_attempts` | exists | roadmap-gap-analysis.md |
| `Limits::max_in_flight` | exists | roadmap-gap-analysis.md |
| `Limits::max_retries` | exists | roadmap-gap-analysis.md |
| `Limits::max_retry_after` | planned (not code yet) | milestones/0.2.md |
| `Limits::request_timeout` | exists | auth.md, security.md |
| `LintWarning` | exists | REPORT.md |
| `ListParams` | exists | architecture.md, interoperability.md, release-readiness.md, roadmap-gap-analysis.md, roadmap.md |
| `ListParams::validate` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `ModelError` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `NotDeclared` | exists | AUDIT.md, audit-report.md, release-readiness.md, security-review.md |
| `NotImplemented` | exists | api-operation-matrix.md |
| `NotPublished` | exists | api-operation-matrix.md, interoperability.md |
| `OrderedPriority` | exists | conformance-gap-analysis.md |
| `Page::new` | exists | AUDIT.md, REPORT.md |
| `PageFollowLimits` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `PageFollowLimits::max_items` | exists | claims-audit.md |
| `PageFollowStop` | exists | roadmap-gap-analysis.md |
| `PageFollowStop::Exhausted` | exists | AUDIT.md |
| `PolicyError` | exists | roadmap-gap-analysis.md |
| `PolicyResolver` | exists | AUDIT.md, security-review.md |
| `PolicyResolver::resolve` | exists | security-review.md |
| `ProductRelease` | exists | roadmap-gap-analysis.md |
| `ProductSort` | exists | roadmap-gap-analysis.md |
| `ProxyMatcher::system()` | exists | AUDIT.md |
| `ReleaseDistribution` | exists | roadmap-gap-analysis.md |
| `ReleaseSort` | exists | roadmap-gap-analysis.md |
| `RepeatedToken` | exists | AUDIT.md |
| `ResolvedRelease` | exists | roadmap-gap-analysis.md |
| `ResolvedRelease::new` | exists | AUDIT.md, REPORT.md |
| `RootReadmeDoctests` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `SelectedEndpoint` | exists | REPORT.md, roadmap-gap-analysis.md |
| `Service::discover_identifier` | exists | api-operation-matrix.md, conformance-gap-analysis.md, requirements-matrix.md |
| `Service::download_artifact` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::download_format` | exists | conformance-gap-analysis.md |
| `Service::download_latest_artifact` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::download_latest_signature` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::download_signature` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::download_url` | exists | roadmap-gap-analysis.md |
| `Service::get_artifact` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_component` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_component_cle` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_component_release` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_component_release_cle` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_component_release_collection` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_latest_artifact` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_latest_component_release_collection` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_latest_product_release_collection` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_product` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_product_cle` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_product_release` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_product_release_cle` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::get_product_release_collection` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::head_artifact` | exists | api-operation-matrix.md, conformance-gap-analysis.md, security-review.md |
| `Service::list_component_release_collections` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::list_component_releases` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::list_product_release_collections` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::list_product_releases` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::page` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `Service::query_component_releases` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::query_components` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::query_product_releases` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `Service::query_products` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `SignatureFetch` | exists | AUDIT.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `SignatureFetch::NotPublished` | exists | interoperability.md |
| `SignatureFetch::Present` | exists | roadmap-gap-analysis.md |
| `SignatureVerifier` | exists | milestones/0.2.md, open-questions.md, roadmap-gap-analysis.md |
| `SignatureVerifyOutcome` | exists | roadmap-gap-analysis.md |
| `SignatureVerifyOutcome::Verified` | exists | AUDIT.md, claims-audit.md, milestones/0.6.md |
| `SlotLease` | exists | AUDIT.md |
| `SortOrder` | exists | roadmap-gap-analysis.md |
| `SortOrder::Extra` | exists | AUDIT.md |
| `StoredToken` | exists | security-review.md |
| `SupportedApi` | exists | architecture.md, roadmap-gap-analysis.md |
| `SupportedApi::supports` | exists | roadmap-gap-analysis.md |
| `SupportedApi::tea_1_0_0` | exists | interoperability-plan.md |
| `SupportedApi::tea_1_0_0()` | exists | roadmap-gap-analysis.md |
| `SupportedApi::with_lenient` | exists | roadmap-gap-analysis.md |
| `TeaErrorBody` | exists | roadmap-gap-analysis.md |
| `TeaErrorCode` | exists | roadmap-gap-analysis.md |
| `TeaUuid::as_uuid()` | exists | roadmap-gap-analysis.md |
| `Tei::parse` | exists | AUDIT.md, REPORT.md, interoperability-plan.md, release-checklist.md |
| `Tei::validate_registry` | exists | conformance-gap-analysis.md, roadmap-gap-analysis.md |
| `Tei::well_known_url` | exists | roadmap-gap-analysis.md |
| `TeiError` | exists | roadmap-gap-analysis.md |
| `TeiError::LegacyUrn` | exists | AUDIT.md, REPORT.md, roadmap-gap-analysis.md |
| `TeiError::Scheme` | exists | REPORT.md, roadmap-gap-analysis.md |
| `TokenResponse` | exists | roadmap-gap-analysis.md |
| `TooLarge` | exists | security-review.md |
| `ValidAndTrusted` | not code | roadmap-gap-analysis.md |
| `ValidButUntrusted` | not code | roadmap-gap-analysis.md |
| `VersionedEndpoint` | exists | AUDIT.md, REPORT.md, roadmap-gap-analysis.md |
| `VersionedEndpoint::new` | exists | AUDIT.md, REPORT.md |
| `WellKnownDocument` | exists | roadmap-gap-analysis.md |
| `WellKnownDocument::parse` | exists | interoperability-plan.md |
| `WellKnownDocument::parse_lenient` | exists | REPORT.md |
| `WellKnownEndpoint` | exists | REPORT.md, roadmap-gap-analysis.md |
| `ZlibInflate` | exists | conformance-gap-analysis.md, security-review.md |
| `a_cancelled_refresh_does_not_wedge_the_next_caller` | exists | AUDIT.md, auth.md |
| `a_cancelled_refresh_leaves_no_slot` | exists | AUDIT.md, auth.md, security.md |
| `a_later_call_retries_after_a_shared_token_failure` | exists | AUDIT.md, auth.md, security.md |
| `a_symlinked_parent_directory_is_followed_by_design` | exists | AUDIT.md |
| `a_wrong_checksum_does_not_write_artifact_bytes` | exists | audit-report.md |
| `absent_identifier_filters_are_omitted` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `add_root_pem` | exists | claims-audit.md, roadmap-gap-analysis.md |
| `allow_discovered_credentials` | exists | interoperability-plan.md, security-review.md, security.md |
| `allow_origin` | exists | interoperability-plan.md, security-review.md |
| `allow_weak_checksums` | exists | AUDIT.md |
| `allows_origin` | exists | audit-report.md, security-review.md |
| `allows_resolved_ip` | exists | requirements-matrix.md, roadmap-gap-analysis.md, security-review.md |
| `artifact_checksum_covers_decoded_bytes_and_not_a_redirect_with_credentials` | exists | api-operation-matrix.md, matrix-audit.md, security-review.md |
| `artifact_latest_versus_revision_and_media_type` | exists | api-operation-matrix.md, matrix-audit.md |
| `artifact_metadata` | exists | audit-report.md, release-readiness.md |
| `artifact_with_url_requires_checksum` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `assert_ne` | exists | AUDIT.md |
| `basic_header` | exists | conformance-gap-analysis.md |
| `bearer_401_does_not_fail_over` | exists | api-operation-matrix.md, conformance-gap-analysis.md, requirements-matrix.md, security-review.md |
| `bearer_challenge_exchanges_an_api_key_once` | exists | conformance-gap-analysis.md, requirements-matrix.md, security-review.md |
| `bearer_for` | exists | conformance-gap-analysis.md, security-review.md |
| `bearer_token_is_kept_under_the_prefix_and_stripped_after_that` | exists | audit-report.md, conformance-gap-analysis.md, requirements-matrix.md, security-review.md |
| `both_identifier_filters_are_encoded_in_the_query` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `check_url` | exists | security-review.md, security.md |
| `checksum_match_decoded_bytes` | exists | claims-audit.md, interoperability.md |
| `checksum_result` | exists | interoperability.md |
| `classifies_embedded_private_addresses` | exists | audit-report.md, requirements-matrix.md, security-review.md |
| `classify_ip` | exists | REPORT.md, requirements-matrix.md, roadmap-gap-analysis.md, security-review.md |
| `classify_v4` | exists | security-review.md |
| `classify_v6` | exists | AUDIT.md |
| `cle_not_implemented_is_distinct_from_an_unknown_object` | exists | api-operation-matrix.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `cle_success_and_object_unknown` | exists | api-operation-matrix.md, matrix-audit.md |
| `cle_success_for_every_cle_endpoint` | exists | api-operation-matrix.md, matrix-audit.md, roadmap-gap-analysis.md |
| `clean()` | exists | AUDIT.md |
| `client_credentials_uses_basic_and_a_form_body` | exists | api-operation-matrix.md, matrix-audit.md |
| `client_for` | exists | security-review.md |
| `compile_fail` | exists | AUDIT.md, REPORT.md, open-questions.md |
| `component_release_collection_latest_and_by_version` | exists | api-operation-matrix.md, matrix-audit.md |
| `component_release_collections_and_versioned_gets` | exists | api-operation-matrix.md, matrix-audit.md |
| `concurrent_challenges_share_one_token_exchange` | exists | auth.md, security.md |
| `concurrent_n32_challenges_one_token_exchange` | exists | REPORT.md, api-operation-matrix.md, auth.md, security.md |
| `conditional_get_304_is_not_a_checksum_and_has_no_body` | exists | api-operation-matrix.md, audit-report.md, conformance-gap-analysis.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `connect_timeout` | exists | roadmap-gap-analysis.md |
| `consumer_surface` | exists | candidate-scope-review.md |
| `content_decoding` | exists | AUDIT.md |
| `content_length()` | exists | security-review.md |
| `core_detect` | external | AUDIT.md |
| `create_new` | exists | AUDIT.md |
| `credential_origins` | exists | AUDIT.md |
| `cybeats_shaped_short_version_fails_by_default_and_works_when_lenient` | exists | roadmap-gap-analysis.md |
| `cybeats_shaped_short_version_is_rejected_by_default` | exists | roadmap-gap-analysis.md |
| `danger_accept_invalid_certs` | absent by design | claims-audit.md |
| `decode_content_coding` | exists | conformance-gap-analysis.md, security-review.md |
| `deny_unknown_fields` | serde attribute | AUDIT.md, REPORT.md, roadmap-gap-analysis.md |
| `discard_token` | removed (historical) | security-review.md |
| `discover_identifier` | exists | AUDIT.md |
| `discovery_403_does_not_fail_over_or_fetch_a_token` | exists | api-operation-matrix.md, conformance-gap-analysis.md, requirements-matrix.md |
| `discovery_accepts_unknown_fields_at_every_level` | exists | roadmap-gap-analysis.md |
| `discovery_follows_version_and_stops_on_tea_404` | exists | api-operation-matrix.md, conformance-gap-analysis.md, matrix-audit.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `discovery_get_collection_artifact_and_cle` | exists | interoperability.md |
| `discovery_json` | exists | audit-report.md, release-readiness.md |
| `discovery_tolerance` | exists | AUDIT.md |
| `does_not_treat_case_variants_as_equal` | exists | conformance-gap-analysis.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `download_artifact` | exists | audit-report.md, release-readiness.md, security-review.md |
| `download_format` | exists | api-operation-matrix.md, audit-report.md, claims-audit.md, conformance-gap-analysis.md, interoperability-plan.md, security-review.md |
| `download_latest_artifact` | exists | conformance-gap-analysis.md, security-review.md |
| `download_latest_artifact_and_signature_aliases` | exists | api-operation-matrix.md, matrix-audit.md |
| `download_url` | exists | security-review.md |
| `empty_discovery_array_is_rejected` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `empty_next_token_is_exhausted_not_followed` | exists | api-operation-matrix.md |
| `empty_page_and_malformed_page_and_oversized_page_size` | exists | api-operation-matrix.md, conformance-gap-analysis.md, roadmap-gap-analysis.md |
| `env_proxy_is_used_by_default_and_skipped_with_ignore_env_proxies` | exists | AUDIT.md, open-questions.md, security.md |
| `escape_control_chars` | exists | AUDIT.md |
| `exception_is_scheme_host_and_port` | exists | audit-report.md, release-readiness.md, requirements-matrix.md, security-review.md |
| `expiry_margin_does_not_consume_a_short_lifetime` | exists | api-operation-matrix.md |
| `fetch_token` | exists | conformance-gap-analysis.md, security-review.md |
| `flate2::Decompress` | external | conformance-gap-analysis.md, security-review.md |
| `flate2::read::DeflateDecoder` | removed (historical) | security-review.md |
| `flate2::write::ZlibDecoder` | removed (historical) | AUDIT.md, security-review.md |
| `follow_pages` | exists | api-operation-matrix.md, interoperability.md, milestones/0.2.md, roadmap-gap-analysis.md, roadmap.md |
| `follow_pages_empty_page_exhausts` | exists | api-operation-matrix.md |
| `follow_pages_hits_max_items` | exists | api-operation-matrix.md |
| `follow_pages_multi_page_then_exhausted` | exists | api-operation-matrix.md |
| `follow_pages_repeated_token_guard` | exists | api-operation-matrix.md |
| `futures_core::Stream` | exists | roadmap-gap-analysis.md |
| `futures_util` | exists | roadmap-gap-analysis.md |
| `get_component` | exists | interoperability.md |
| `get_component_happy_path` | exists | REPORT.md, api-operation-matrix.md, matrix-audit.md |
| `get_component_release` | exists | interoperability.md |
| `get_product` | exists | interoperability.md |
| `get_product_release` | exists | roadmap-gap-analysis.md |
| `gets_a_product` | exists | api-operation-matrix.md, conformance-gap-analysis.md, matrix-audit.md |
| `gzip_expansion_is_capped` | exists | AUDIT.md, audit-report.md, security-review.md |
| `hanging_token_endpoint_times_out_without_the_secret` | exists | AUDIT.md, api-operation-matrix.md, auth.md, roadmap-gap-analysis.md |
| `hanging_token_origin_does_not_stall_waiters_or_other_origins` | exists | AUDIT.md, REPORT.md, api-operation-matrix.md, auth.md, security.md |
| `has_next` | exists | interoperability.md |
| `hasher_for` | exists | AUDIT.md |
| `head_artifact` | exists | audit-report.md, security-review.md |
| `head_bearer_challenge_is_an_authentication_error` | exists | api-operation-matrix.md |
| `head_ignores_representation_content_length` | exists | api-operation-matrix.md, audit-report.md, conformance-gap-analysis.md, matrix-audit.md, release-readiness.md |
| `head_redirect_uses_the_final_headers_and_strips_a_real_token` | exists | audit-report.md, conformance-gap-analysis.md |
| `head_without_content_length_returns_metadata` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `help_text_says_only_the_final_path_component_is_checked` | exists | AUDIT.md |
| `host_excepted` | removed (historical) | security-review.md |
| `http_deflate_is_zlib_and_raw_deflate_is_rejected` | exists | audit-report.md, conformance-gap-analysis.md |
| `icu_normalizer` | external | open-questions.md |
| `icu_properties` | external | AUDIT.md, open-questions.md |
| `id_type_without_id_value_is_accepted` | exists | requirements-matrix.md |
| `id_value` | exists | roadmap-gap-analysis.md |
| `idna_adapter` | external | AUDIT.md, REPORT.md, open-questions.md, release-readiness.md |
| `if_none_match` | exists | roadmap-gap-analysis.md |
| `ignore_env_proxies` | exists | roadmap-gap-analysis.md |
| `initial_backoff` | exists | roadmap-gap-analysis.md |
| `into_inner` | exists | AUDIT.md |
| `is_certification` | exists | AUDIT.md, claims-audit.md |
| `is_multiple_of` | exists | AUDIT.md, REPORT.md |
| `is_unspecified` | exists | security-review.md |
| `json_error_text_is_clean` | exists | AUDIT.md |
| `keeps_build_metadata_spelling` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `lazy_static` | external | AUDIT.md |
| `legacy_urn_form_has_a_distinct_error` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `lenient_versions` | exists | AUDIT.md, REPORT.md, milestones/0.2.md, milestones/1.0.md |
| `libtea_client` | exists | release-readiness.md, requirements-matrix.md |
| `libtea_client::Client::discover` | exists | requirements-matrix.md |
| `libtea_client::url` | exists | REPORT.md, roadmap-gap-analysis.md |
| `libtea_conformance::static_report` | exists | conformance-gap-analysis.md |
| `libtea_core` | exists | AUDIT.md, release-readiness.md |
| `libtea_core::DiscoveryInfo` | exists | requirements-matrix.md |
| `libtea_core::Tei` | exists | requirements-matrix.md |
| `libtea_core::Tei::parse` | exists | conformance-gap-analysis.md |
| `libtea_core::discovery` | exists | requirements-matrix.md |
| `libtea_core::escape_control_chars` | exists | AUDIT.md, open-questions.md |
| `libtea_core::ids` | exists | requirements-matrix.md |
| `libtea_core::model` | exists | requirements-matrix.md |
| `libtea_core::semver` | exists | roadmap-gap-analysis.md |
| `libtea_core::spec` | exists | re-pinning.md |
| `libtea_core::url` | exists | roadmap-gap-analysis.md |
| `libtea_core::uuid` | exists | roadmap-gap-analysis.md |
| `libtea_core::verify_checksums` | exists | requirements-matrix.md |
| `lint_discovery_strict` | exists | REPORT.md, roadmap-gap-analysis.md |
| `lint_well_known_strict` | exists | REPORT.md, roadmap-gap-analysis.md |
| `list_and_query_component_releases_paths` | exists | api-operation-matrix.md, matrix-audit.md |
| `list_and_query_product_releases_paths_and_params` | exists | api-operation-matrix.md, matrix-audit.md |
| `list_component_release_collections` | exists | interoperability.md |
| `manual_is_multiple_of` | external | AUDIT.md |
| `max_artifact_bytes` | exists | AUDIT.md, security-review.md |
| `max_discovery_attempts` | exists | AUDIT.md |
| `max_in_flight` | exists | AUDIT.md, milestones/0.3.md, roadmap-gap-analysis.md, security-review.md, security.md |
| `max_items` | exists | AUDIT.md, interoperability.md |
| `max_json_bytes` | exists | AUDIT.md |
| `max_output` | exists | security-review.md |
| `max_pages` | exists | interoperability.md |
| `max_redirects` | exists | AUDIT.md |
| `max_retries` | exists | milestones/0.2.md, security-review.md |
| `min_tls_version` | exists | conformance-gap-analysis.md |
| `missing_and_unknown_algorithms_are_not_verified` | exists | requirements-matrix.md |
| `next_page_token` | exists | REPORT.md |
| `no_proxy()` | exists | AUDIT.md |
| `no_run` | not code | roadmap-gap-analysis.md |
| `no_std` | not code | roadmap.md, specification-baseline.md |
| `no_such_option` | not code | AUDIT.md |
| `non_exhaustive` | exists | milestones/0.2.md, milestones/1.0.md, roadmap-gap-analysis.md |
| `non_tea_404_fails_over_to_the_next_endpoint` | exists | api-operation-matrix.md, conformance-gap-analysis.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `not_acceptable_is_an_api_error_and_a_missing_json_type_is_rejected` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `not_confirmed` | exists | interoperability.md |
| `omitted_priority_is_one_and_http_base_parses` | exists | conformance-gap-analysis.md |
| `oolong_idtype_live` | exists | AUDIT.md, REPORT.md, api-operation-matrix.md, conformance-gap-analysis.md, conformance.md, interoperability.md, requirements-matrix.md |
| `oolong_surface_live` | exists | AUDIT.md, REPORT.md, api-operation-matrix.md, interoperability.md |
| `operation_url` | exists | conformance-gap-analysis.md |
| `origin_key` | exists | roadmap-gap-analysis.md |
| `origin_policy` | exists | audit-report.md, release-readiness.md |
| `other_bad_schemes_keep_the_scheme_error` | exists | AUDIT.md |
| `other_majors_are_rejected_even_when_lenient` | exists | roadmap-gap-analysis.md |
| `oversized_json_is_rejected` | exists | api-operation-matrix.md, matrix-audit.md, requirements-matrix.md, security-review.md |
| `page_stream` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `page_stream::tests::empty_next_token_is_exhausted_not_followed` | exists | AUDIT.md |
| `paginate_components` | exists | dependency-weight.md |
| `pagination_anomalies_are_visible_to_callers` | exists | api-operation-matrix.md, matrix-audit.md |
| `parse_api_base` | exists | roadmap-gap-analysis.md |
| `parse_tei` | exists | roadmap-gap-analysis.md |
| `parser_unicode` | exists | AUDIT.md |
| `parses_registry_examples` | exists | conformance-gap-analysis.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `policy::tests::deprecated_ipv6_forms_are_not_public` | exists | AUDIT.md |
| `prefers_higher_version_over_priority` | exists | conformance-gap-analysis.md, roadmap-gap-analysis.md |
| `prerelease_is_not_the_stable_version` | exists | conformance-gap-analysis.md |
| `preserves_unknown_artifact_type` | exists | api-operation-matrix.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `probe_redacts_a_secret_echoed_by_the_server_and_strips_url_parts` | exists | AUDIT.md |
| `probe_reports_weak_only_checksums_by_exit_status` | exists | AUDIT.md, open-questions.md |
| `probe_runs_end_to_end_and_keeps_secrets_out_of_results` | exists | AUDIT.md |
| `probe_with_nothing_to_do_fails_instead_of_passing` | exists | AUDIT.md |
| `product_pages_keep_tokens_opaque_and_do_not_follow_urls` | exists | api-operation-matrix.md, conformance-gap-analysis.md, matrix-audit.md |
| `product_release_collection_latest_and_by_version` | exists | api-operation-matrix.md, matrix-audit.md |
| `product_release_get_and_unknown_identifier` | exists | api-operation-matrix.md, matrix-audit.md |
| `production_policy_rejects_cleartext_and_metadata` | exists | conformance-gap-analysis.md, requirements-matrix.md, security-review.md |
| `pull_request_target` | not code | AUDIT.md |
| `query_encoding_matches_openapi_example_shape` | exists | conformance-gap-analysis.md |
| `query_pairs_mut` | exists | conformance-gap-analysis.md |
| `query_products` | exists | interoperability.md |
| `rand_chacha` | external | AUDIT.md |
| `rand_core` | external | AUDIT.md |
| `read_decoded` | exists | AUDIT.md, roadmap-gap-analysis.md, security-review.md |
| `read_limited` | exists | security-review.md |
| `redact_url_for_log` | exists | roadmap-gap-analysis.md |
| `redirect_limit_and_gzip_expansion_limit_are_enforced` | exists | AUDIT.md, api-operation-matrix.md, audit-report.md, roadmap-gap-analysis.md, security-review.md |
| `redirect_to_another_origin_is_refused_before_a_credential_is_sent` | exists | conformance-gap-analysis.md, requirements-matrix.md, security-review.md |
| `reexported_url_is_the_type_the_api_takes` | exists | AUDIT.md |
| `refresh_slots_do_not_grow_with_origin_churn` | exists | AUDIT.md, auth.md, security.md |
| `refresh_token_coalesced` | exists | AUDIT.md |
| `rejected_bearer_is_dropped_after_one_retry` | exists | api-operation-matrix.md, security-review.md |
| `rejects_slash_and_port` | exists | conformance-gap-analysis.md, requirements-matrix.md |
| `report_is_not_a_certification_and_names_the_pinned_commit` | exists | AUDIT.md, claims-audit.md |
| `request_timeout` | exists | AUDIT.md, auth.md, roadmap-gap-analysis.md |
| `require_checksums_rejects_a_format_without_checksums` | exists | AUDIT.md |
| `reqwest::Error` | external | security.md |
| `reqwest::Response::content_length` | external | security-review.md |
| `resolve_tests` | exists | roadmap-gap-analysis.md |
| `respects_max_pages` | exists | api-operation-matrix.md |
| `same_major_is_selected_only_when_lenient` | exists | roadmap-gap-analysis.md |
| `same_major_versions_need_the_flag` | exists | roadmap-gap-analysis.md |
| `select_endpoint` | exists | roadmap-gap-analysis.md |
| `select_endpoints` | exists | conformance-gap-analysis.md, roadmap-gap-analysis.md |
| `selects_the_normative_well_known_example` | exists | requirements-matrix.md |
| `semver::Version` | external | roadmap-gap-analysis.md |
| `semver::Version::parse` | external | interoperability-plan.md |
| `send_once` | exists | security-review.md |
| `serde_json` | exists | AUDIT.md, roadmap-gap-analysis.md |
| `server_base_url` | exists | AUDIT.md |
| `sha1_only_match_is_not_verified_by_default` | exists | AUDIT.md |
| `sha256_matches_and_mismatch_is_not_success` | exists | requirements-matrix.md |
| `shared_failure_does_not_outlive_its_burst` | exists | AUDIT.md, auth.md |
| `short_lifetime_is_still_presented` | exists | api-operation-matrix.md |
| `should_refresh_token` | exists | conformance-gap-analysis.md |
| `signature_absence_is_not_verification_and_unknown_is_an_error` | exists | api-operation-matrix.md, requirements-matrix.md |
| `signature_bytes_are_not_reported_as_verified` | exists | api-operation-matrix.md, matrix-audit.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `spec::COMMIT` | exists | roadmap-gap-analysis.md |
| `stale_401_keeps_a_newer_cached_token` | exists | security-review.md |
| `static_report()` | exists | roadmap-gap-analysis.md |
| `std::fs::write` | external | AUDIT.md |
| `std::net::IpAddr` | external | roadmap-gap-analysis.md |
| `std::sync::Mutex` | external | AUDIT.md, security-review.md |
| `stdout_contains_only_verified_artifact_bytes` | exists | audit-report.md |
| `stops_on_repeated_token` | exists | api-operation-matrix.md |
| `strict_lint_is_quiet_for_valid_documents_and_bad_json` | exists | AUDIT.md |
| `strict_lint_warns_about_unknown_fields_and_never_fails_the_report` | exists | AUDIT.md |
| `symlink_metadata` | exists | AUDIT.md |
| `tea_1_0_0()` | exists | roadmap-gap-analysis.md |
| `tea_client::Service` | exists | audit-report.md |
| `tea_error` | exists | AUDIT.md |
| `tea_error_code` | exists | audit-report.md, conformance-gap-analysis.md |
| `tei::tests::non_ascii_after_percent_is_an_error_not_a_panic` | exists | AUDIT.md |
| `tei_identifier_validation_does_not_panic` | exists | AUDIT.md |
| `tei_parse` | exists | AUDIT.md, audit-report.md, release-readiness.md |
| `text_helpers_do_not_panic_on_unicode` | exists | AUDIT.md |
| `timestamp_utc` | exists | milestones/0.1.md |
| `token_endpoint_not_implemented_is_server_limitation` | exists | interoperability.md |
| `token_error_text_omits_the_secret` | exists | AUDIT.md, api-operation-matrix.md, auth.md |
| `token_refresh` | exists | AUDIT.md, claims-audit.md |
| `tokio::Mutex` | external | AUDIT.md |
| `toml_datetime` | external | AUDIT.md |
| `toml_edit` | external | AUDIT.md |
| `toml_parser` | external | AUDIT.md |
| `truncated_gzip_and_zlib_are_rejected` | exists | security-review.md |
| `type_only_filter_stays_on_the_next_page` | exists | api-operation-matrix.md, conformance-gap-analysis.md, matrix-audit.md |
| `type_only_identifier_filter_is_serialized` | exists | api-operation-matrix.md, conformance-gap-analysis.md, requirements-matrix.md, roadmap-gap-analysis.md |
| `unknown_error_is_still_a_tea_error` | exists | api-operation-matrix.md |
| `unknown_type_parses_without_registry_rules` | exists | conformance-gap-analysis.md |
| `url::Url` | external | roadmap-gap-analysis.md |
| `url_redaction_tests` | exists | roadmap-gap-analysis.md, security.md |
| `url_under_prefix` | exists | conformance-gap-analysis.md, security-review.md |
| `uuid::Uuid` | external | roadmap-gap-analysis.md |
| `validate_purl` | exists | milestones/0.4.md, roadmap-gap-analysis.md |
| `value_only_identifier_filter_sends_no_request` | exists | api-operation-matrix.md, conformance-gap-analysis.md |
| `verify_checksums` | exists | conformance-gap-analysis.md, security-review.md |
| `verify_checksums_with_policy` | exists | REPORT.md, roadmap-gap-analysis.md |
| `version_negotiation` | exists | audit-report.md, release-readiness.md |
| `versioned_api_prefix` | exists | conformance-gap-analysis.md, roadmap-gap-analysis.md |
| `weak_algorithms_only` | exists | AUDIT.md, interoperability.md |
| `weak_checksum_behavior::sha1_only_match_is_not_verified_by_default` | exists | AUDIT.md |
| `weak_only_checksums_exit_4_unless_allowed` | exists | AUDIT.md, open-questions.md, roadmap-gap-analysis.md |
| `well_known_accepts_unknown_fields_at_every_level` | exists | roadmap-gap-analysis.md |
| `well_known_fetch_url` | exists | audit-report.md |
| `with_query` | exists | roadmap-gap-analysis.md |
| `without_url` | exists | security.md |
| `workflow_dispatch` | exists | interoperability.md, open-questions.md |
| `workflow_policy` | exists | AUDIT.md |
| `write_all` | exists | security-review.md |
| `write_output` | exists | AUDIT.md |
| `zero_lifetime_is_not_presented` | exists | api-operation-matrix.md |
| `zlib_deflate_is_decoded_before_the_checksum` | exists | api-operation-matrix.md, audit-report.md, conformance-gap-analysis.md, matrix-audit.md, release-readiness.md, requirements-matrix.md, security-review.md |
