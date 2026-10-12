# Security review

Date: 2026-10-10. Scope: `tea-core` destination policy, `tea-client` transport and downloads, credential handling, and the CLI. No implementation file was changed. Findings below are from reading the pinned sources and from the tests that actually ran. Those package names are the names from this review. The current names are `cdx-tea-core`, `cdx-tea-client`, `cdx-tea-conformance`, and `cdx-tea-cli`. Directory paths did not change.

Remediation after this review added the origin-scoped DNS check, zlib `deflate`, HEAD handling, and a mock test that strips a real bearer token off the API prefix. Current behavior is summarized in [security.md](security.md). A matching checksum is still not detached-signature verification or publisher authentication. The paragraphs below keep the 2026-10-10 findings, including notes that were true before those tests existed.

Production defaults refuse cleartext, userinfo, literal loopback, literal link-local (`169.254.169.254` included), and a custom reqwest resolver that drops non-public answers unless the hostname is excepted. TLS verification is rustls with the webpki roots. There is no switch that disables certificate checks. Tokens and API-key secrets are redacted in `Debug`. Request logs record the URL and status, not the `Authorization` value. `reqwest` is built without the gzip feature, so content coding is not decoded twice by the HTTP stack.

Those defaults were not exercised with a live DNS rebinding harness or a hostile certificate.

## 2026-10-11 regression pass

A later pass re-read the transport after the token-cache change in `3da0ba7`. Two defects were confirmed and fixed. This is not a new certification of the 2026-10-10 findings.

`discard_token` removed the origin's cache entry after a rejected retry without comparing it to the bearer token that retry had sent. A delayed `401` for an older token deleted a newer token stored by another request. `stale_401_keeps_a_newer_cached_token` orders that race with a local server and a oneshot, and fails on the old removal. Invalidation now removes the entry only when the stored value is still the presented token. `rejected_bearer_is_dropped_after_one_retry` still requires that a rejected token which is still current is not reused.

`ContentDecoder` checked the decoded length after `write_all` returned, so one compressed chunk could expand the buffer past the cap before the error. The decoder writer now refuses that write. `flate2`'s zlib writer also returned `Ok` for a truncated member: cutting `artifact-bytes` in half came back as `Ok(b"artifact")`. HTTP `deflate` now requires zlib stream end, so that partial member is an error. `gzip_expansion_is_capped` still rejects the oversize output. `truncated_gzip_and_zlib_are_rejected` covers a cut-off gzip member, a cut-off zlib member, and a zlib member with the Adler-32 trailer removed.

No live DNS rebinding harness was added. The connection uses the addresses `PolicyResolver` returned; see `docs/security.md`.

## TEA-AUD-003 — Origin exceptions are hostname-wide at DNS time

Status after remediation: **Verified**. `host_excepted` is gone. `PolicyResolver` allows a non-public answer only when `allows_resolved_ip` accepts the request URL captured in `REQUEST_URL`. Outside that task, only public addresses are kept. `exception_is_scheme_host_and_port` allows `http://allowed.test:8080` for `10.1.2.3` and `fd00::1`, and refuses the same host on `https` and on port `8443`, and a different host on the same addresses. Default-port keys match (`https://example.test` and `:443`). IPv6 literals use brackets. A live DNS rebinding process was not run; the decision function is deterministic.

Severity: High. Class: implementation defect.

Affected code:

- `crates/tea-core/src/policy.rs`, `DestinationPolicy::host_excepted` (hostname comparison only)
- `crates/tea-client/src/transport.rs`, `PolicyResolver::resolve` (allows any address when `host_excepted` is true)

Normative context: the discovery document requires certificate verification and forbids forwarding credentials on the basis of a redirect. This SDK additionally documents that `allow_origin` applies to one `scheme://host:port`. `allows_origin` implements that key. The resolver does not.

Evidence: `check_url` calls `allows_resolved_ip`, which permits a non-public address only when the full origin matches. `PolicyResolver` instead calls `host_excepted`, which returns true for every exception whose hostname matches, ignoring scheme and port. A caller who excepts `http://dev.example.com:8080` therefore accepts a private or link-local address for `https://dev.example.com` as well.

Impact: an exception intended for one local origin becomes a DNS-rebinding and SSRF exception for every URL on that name. The default policy, with an empty exception set, does not take this branch.

Reproduction:

1. Build a client with `DestinationPolicy::allow_origin` for `http://dev.example.com:8080` only.
2. Resolve `https://dev.example.com/.well-known/tea` through `PolicyResolver` while `dev.example.com` answers `127.0.0.1` or `169.254.169.254`.
3. Observe that the private address is kept.

Suggested test: two origins, same hostname, different ports. The excepted origin accepts its own loopback literal. The other scheme and port must still be refused when the resolver returns a non-public address. Do not widen the default policy to make the test pass.

Remediation: in the resolver, allow a non-public answer only when the URL being resolved has an excepted origin key. Pass that URL, or scheme and port, into the check. `host_excepted` should not be the connect decision.

## TEA-AUD-001 — `deflate` is not HTTP deflate

Status after remediation: **Verified**. `deflate` uses `flate2::write::ZlibDecoder`. Gzip still uses `GzDecoder`. Raw DEFLATE is an error. `zlib_deflate_is_decoded_before_the_checksum` checks SHA-256 of the decoded bytes. Input and output are both capped.

Severity: High. Class: implementation defect. Also an interoperability defect.

Affected code at the time of the finding: `crates/tea-core/src/checksum.rs`, `decode_content_coding`, the `deflate` branch that constructed `flate2::read::DeflateDecoder`. The status line above is the code after the fix.

Requirement: `tea-collection/tea-collection.md` says clients shall remove HTTP content coding before comparing checksums. RFC 9110 defines the `deflate` coding as the zlib format (RFC 1950), not raw DEFLATE (RFC 1951). `DeflateDecoder` is raw DEFLATE.

Impact: a server that sends a spec-compliant `Content-Encoding: deflate` body fails decoding, or, if a raw stream is accepted from a non-compliant peer, the checksum is computed over the wrong framing. Gzip uses `GzDecoder` and has a unit test. Deflate has no test. The decoded-size cap does apply to both, so this is not an unbounded decompression bomb. Compressed input is also capped by `max_artifact_bytes` before decode, and output is capped again.

Reproduction, against the code before the fix: zlib-wrap the bytes `artifact-bytes` and call `decode_content_coding(body, Some("deflate"), 1024)`. A correct decoder returns `artifact-bytes`. The decoder at audit time returned an error or different bytes.

Remediation: decode `deflate` with a zlib decoder. Keep rejecting stacked codings and unknown codings. Add the vector above and a truncated zlib member that must not return `IntegrityOutcome::Verified`.

## TEA-AUD-002 — `HEAD` treats `Content-Length` as a body

Status after remediation: **Verified**. `HEAD` and status `304` drop the response without reading a body and without using `Content-Length` as a budget. Regression tests cover a non-zero `Content-Length`, a missing `Content-Length`, `ETag`, `Content-Type`, a bearer `401`, and a redirect that strips a real token.

Severity: High. Class: implementation defect.

Affected code:

- `crates/tea-client/src/service.rs`, `Service::head_artifact`, which sets `max_body: 0`
- `crates/tea-client/src/transport.rs`, `read_limited`, which returns `TooLarge` when `content_length()` is greater than `max`

Requirement: the pinned download operation says a server that supports `HEAD` shall return the same headers as `GET` and no response body. Those headers include `Content-Length` of the selected representation. `reqwest::Response::content_length` reports that header.

Impact: `head_artifact` fails for every artifact whose `Content-Length` is greater than zero, which is the normal case. Callers cannot use `HEAD` to learn `ETag` or `Content-Type`.

Reproduction: mock `HEAD /v1.0.0/artifact/{id}/1/download` with `Content-Length: 4`, `ETag: "abc"`, and an empty body. `head_artifact` returns `ClientError::TooLarge`.

Remediation: for `HEAD`, ignore `Content-Length` as a body budget and read at most a small discard limit. Assert the returned map contains the `ETag` and that `TooLarge` is not returned.

## TEA-AUD-004 — Redirect token stripping is not demonstrated

Status after remediation: **Verified**. `bearer_token_is_kept_under_the_prefix_and_stripped_after_that` fetches a real token, sends it on the versioned download and on a same-origin path still under `/v1.0.0/`, and fails if `/files/outside` receives `Authorization`. A second redirect cannot put the header back. `redirect_to_another_origin_is_refused_before_a_credential_is_sent` returns `ClientError::Policy` and the other origin expects zero requests. Tests do not print the token.

Severity: Medium. Class: test gap. The code path is plausible and was not shown to fail.

Affected test: `crates/tea-client/tests/http.rs`, `artifact_checksum_covers_decoded_bytes_and_not_a_redirect_with_credentials`. It mounts a `500` if `Authorization` is present on `/files/sbom`. The client is `client_for`, which does not set an API key. No bearer token exists to leak.

Affected code: `bearer_for` returns a token only when `url_under_prefix` is true. A redirect from `/v1.0.0/.../download` to `/files/sbom` is outside the prefix, so the next request should omit the header. That branch did not run with a stored token.

Requirement: discovery/readme.md says clients shall not forward a TEA access token outside the authorized API base, and collection text says a token is never sent to an external URL.

Impact of the gap: a regression that attached `Authorization` on that redirect would still pass this test.

Reproduction of the missing assertion:

1. Configure an API key and a token endpoint that returns `access_token` `tok`.
2. `GET` a versioned download with a bearer challenge so the token is cached.
3. Redirect to `/files/sbom` on the same host.
4. Fail the test if that request contains `Authorization`.

## TEA-AUD-005 — CLI download does not verify checksums

Status after remediation: **Verified**. The CLI loads metadata and calls `download_format`. A wrong checksum exits non-zero with empty stdout. A matching checksum writes only the artifact bytes to stdout and the status line to stderr. `NotDeclared` is reported as not verified. The CLI does not claim `Verified` when no checksum matched.

Severity: Medium. Class: implementation defect in the CLI, not in the checksum function.

Affected code: `crates/tea-cli/src/main.rs`, the `Download` arm. It calls `download_artifact` and `download_latest_artifact` with `&[]`. `download_url` treats `IntegrityOutcome::NotDeclared` as success and returns the bytes. The CLI prints an integrity line and then writes those bytes. With no `--output`, the line is written to stdout before the artifact, so a pipe is not the artifact.

Impact: `tea download` reports an integrity outcome and saves bytes that were not checked. The outcome will be `NotDeclared`.

Remediation: require a prior metadata fetch and `download_format`, or require the caller to pass checksums. Write status to stderr. Suggested test: CLI download of a mock artifact with a wrong published checksum exits non-zero and does not write the output file.

## TEA-AUD-006 — Artifact downloads are fully buffered

Status after remediation: **Partially verified**. `read_decoded` applies `ContentDecoder` per chunk, so the encoded body is not retained as a second buffer. The decoded bytes are still held up to `max_artifact_bytes` so checksums can be compared. `gzip_expansion_is_capped` and `redirect_limit_and_gzip_expansion_limit_are_enforced` reject expansion past the cap. Redirects are bounded. GET retries stop at `max_retries`. Pagination is one request per call.

Severity: Medium. Class: architecture and resource limit, not a spec miss.

Affected code: `read_limited` accumulates the encoded body, then `decode_content_coding` allocates the decoded body, then `verify_checksums` hashes that buffer. `ChecksumVerifier::update` could hash incrementally and is unused on this path.

The default cap is 256 MiB for the encoded body and 256 MiB for the decoded body. A gzip or zlib bomb is stopped at the output cap. Memory can still reach about two times the cap for one download. The semaphore bounds concurrent downloads (`max_in_flight` default 8), so the worst case is large.

Remediation: hash while decoding and enforce the output cap without retaining both copies longer than needed. This is not a release blocker if the caps stay.

## TEA-AUD-007 — Some non-routable IPv4 ranges are classified public

Status after remediation: **Verified**. `classify_v4` maps `0.0.0.0/8` to `Reserved` and `198.18.0.0/15` to `Documentation`. `classifies_embedded_private_addresses` asserts `0.1.2.3` and `198.18.0.1`.

Severity: Low.

Affected code: `classify_v4` in `crates/tea-core/src/policy.rs`. `is_unspecified` is only `0.0.0.0`. Other addresses in `0.0.0.0/8` fall through to `Public`. `198.18.0.0/15` (benchmarking) is not classified.

Impact: a URL whose literal or resolved address is in those ranges is allowed by the default policy. This is a smaller SSRF surface than link-local or loopback, which are refused.

Suggested test: `classify_ip` of `0.0.0.1` and `198.18.0.1` is not `Public`.

## TEA-AUD-008 — Well-known URL override skips the TEI host

Severity: Low.

Affected code: `ClientBuilder::well_known_fetch_url`. Discovery then fetches that URL instead of `https://{tei-domain}/.well-known/tea`. Certificate checks apply to the override host, not to the TEI domain. The method is public and documented as a fixture hook.

Impact: a production caller can bind a TEI to a document served by a different name. The default path does not do this.

Remediation: keep the hook for tests, and say in the API docs that it disables the TEI-domain identity check. No code change is required for the default path.

## Controls that held under review

| Control | Evidence | Verification |
| --- | --- | --- |
| No invalid-certificate option | `ClientBuilder::build` sets rustls and TLS 1.2 minimum | Code only |
| Default refusal of `http://example.com` and `https://169.254.169.254` | `production_policy_rejects_cleartext_and_metadata` | Verified |
| IPv4-mapped loopback and NAT64 `64:ff9b::/96` embedding `10.0.0.1` | `classifies_embedded_private_addresses` | Verified |
| Userinfo rejected | `check_url` | Code only |
| POST `/token` is not redirected | `send_once` skips redirects when the method is POST | Code only |
| JSON and discovery size limits | `oversized_json_is_rejected` for JSON. Discovery cap is code only | Partial |
| Secret redaction | `ApiKey` and `StoredToken` `Debug` | Code only |
| Credential not sent to discovered hosts unless opted in | `allow_discovered_credentials` default false; `service` authorizes the caller-supplied origin | Code only |
| One token fetch after a bearer `401` | `bearer_challenge_exchanges_an_api_key_once` | Verified |
| `401` does not continue to the next discovery endpoint | `bearer_401_does_not_fail_over` | Verified |
| Gzip bomb bounded by `max_output` | `read_limited` in `checksum.rs` stops when output would exceed the cap | Code only. No bomb fixture was run |
| Retry of GET on transport failure and HTTP 503 | `send` | Code only |

`std::sync::Mutex` around the token map is not held across `.await`. A cancellation during `fetch_token` happens before the token is inserted. That part is cancellation-safe. It was not stress-tested.
