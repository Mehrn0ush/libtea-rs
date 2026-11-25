# Security

## Assets

- API key identifier and secret, and the bearer token derived from them
- Artifact bytes and the checksum result that callers use to accept those bytes
- The TEI and the API base a caller decided to trust
- Local resources reachable if a server returns a URL the client then fetches

## Trust boundaries

The caller trusts the initial TEI domain enough to request its well-known document, subject to the destination policy. Endpoints listed in that document are not automatically trusted with credentials. `allow_discovered_credentials` must be set before an API key is posted to a host that was not the origin the caller authorized through `Client::service` or `Client::discover`.

Artifact `url` and `signatureUrl` values are third-party fetches. They never receive the bearer token. Redirects are followed manually, up to the configured limit, and the token is attached only when the next URL is under the versioned API prefix on an authorized origin.

## Authentication

The API key is form-urlencoded and then placed in the Basic credential for `POST /token`. Resource requests use `Authorization: Bearer`. Tokens are not logged. `Debug` for `ApiKey` and the token cache redacts the secret and the token. Debug request and response logs print URLs with userinfo and fragments removed and query values replaced by `redacted`. `ClientError::transport` drops the structured URL on a `reqwest::Error` (`without_url`) before storing text in `ClientError::Transport`. Residual absolute URLs in nested error text are rewritten only when the full whitespace-bounded token parses as a URL; otherwise the message is replaced with `request failed`. Quote boundaries and trailing punctuation are not treated as safe delimiters. Unit coverage is `url_redaction_tests` in `transport.rs`.

A `401` with `WWW-Authenticate: Bearer`, with no error or with `error=invalid_token`, obtains a token and retries the resource request once. A challenge with another `error` value, including `insufficient_scope`, is not retried. If the retry is still `401`, the bearer token that retry presented is removed only when it is still the cached value for that origin. A newer token stored by another request is left in place. `403` is returned to the caller. The client does not call `/token` before a resource asks for a token. `POST /token` is not followed if it redirects.

The request body is only `grant_type=client_credentials`. No `scope` or `audience` parameter is sent. A `scope` on the token response is ignored. When `expires_in` is greater than five seconds, the cached token is treated as expired five seconds early. A shorter positive lifetime is kept for its full `expires_in`. `expires_in` of `0` is already expired. If `expires_in` is omitted, the token stays cached until the server rejects it.

**Token-refresh serialization:** refresh uses one async lock **per API origin**, created on first use for an origin the key may be sent to. Behavior:

1. A challenge that should refresh acquires that origin's lock before `POST /token`.
2. After waiting, if the cache already holds a usable bearer different from the value that received `401` (or any bearer when none was presented), that token is reused and no additional exchange runs.
3. Otherwise, if an exchange for that origin finished while this caller was queued and it failed, the caller returns the same failure.
4. Otherwise the client exchanges the API key once under the lock and stores the result for that origin.

Different origins do not wait for each other. An origin's lock entry exists only while a refresh for it is running or queued; the last holder removes it, including when it is cancelled, so many distinct origins cannot grow the map (`refresh_slots_do_not_grow_with_origin_churn`, `a_cancelled_refresh_leaves_no_slot`). A hanging `/token` is bounded by `Limits::request_timeout`; queued same-origin waiters share that one timeout. This is a per-burst bound, not exactly one `POST /token` for every concurrent set: see `docs/auth.md` for when more exchanges happen. Ordinary resource GETs are not serialized by these locks. Regressions: `concurrent_challenges_share_one_token_exchange`, `concurrent_n32_challenges_one_token_exchange`, `hanging_token_origin_does_not_stall_waiters_or_other_origins`, `a_later_call_retries_after_a_shared_token_failure`. The token cache key remains the origin (scheme, host, and port). Two version prefixes on that origin share the cached token, and the token is attached only when the request URL is under the prefix for that call.

### CLI credentials

Prefer `TEA_API_KEY_ID` and `TEA_API_KEY_SECRET`. A complete `--api-key-id` / `--api-key-secret` pair still works for compatibility, but placing the secret on the command line exposes it to process listings and shell history. Partial pairs and mixing a CLI id with an environment secret (or the reverse) are rejected. The CLI never prints the secret. Unit tests that temporarily clear those variables restore the previous process environment on exit so `cargo test` does not erase a developer's configured keys.

## Transport

reqwest is built with rustls. Certificate verification uses the webpki roots and any extra PEM roots the caller adds. There is no option to disable verification. The minimum TLS version is 1.2.

The default `DestinationPolicy` allows HTTPS to public addresses. It refuses HTTP, loopback, link-local, unique-local, private, CGNAT, documentation, benchmarking (`198.18.0.0/15`), `0.0.0.0/8`, and cloud-metadata addresses. Hostnames `localhost`, names under `.local` and `.localhost`, and the Google metadata hostnames are refused.

An exception from `DestinationPolicy::allow_origin` is one origin: scheme, hostname, and effective port. `https://example.test` and `https://example.test:443` are the same origin. `http://example.test`, `https://example.test:8443`, and a different hostname are not. Matching an origin does not authorize every address that hostname can resolve to on another scheme or port. A different hostname that resolves to the same address is still refused unless that second origin is excepted on its own.

DNS answers are filtered for the URL of the request in progress. A non-public answer is kept only when that exact origin is excepted. If the resolver is not running inside a request, only public addresses are kept. IPv4-mapped IPv6, 6to4, and NAT64 (`64:ff9b::/96`) are classified by the embedded IPv4 address. The filtered addresses are what the HTTP client connects to: reqwest does not look the name up again for that connection. A literal IP skips the resolver and is classified by `check_url` before the request is sent. This has been checked from the reqwest and hyper connection code. It has not been exercised with a hostile DNS server that changes its answer between lookups. IPv4-compatible `::a.b.c.d`, site-local `fec0::/10`, and NAT64 local-use `64:ff9b:1::/48` are not public.

**Proxies:** environment proxies (`HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, lowercase forms, and `NO_PROXY`) are honored by default, as reqwest does. Through a proxy, the client connects to the proxy and the **proxy resolves the target hostname**, so the DNS-answer filtering above is **not enforced** for hostname URLs: a discovered `https://internal.example` that resolves to `10.0.0.5` is fetched by the proxy. Scheme, userinfo, literal-IP, blocked-hostname, and per-hop redirect checks still run, and bearer tokens stay scoped to the API prefix. The proxy address itself is not checked against the policy. `ClientBuilder::ignore_env_proxies(true)` makes the client connect directly and keeps the DNS checks; use it for clients that follow discovered URLs. Regression: `env_proxy_is_used_by_default_and_skipped_with_ignore_env_proxies` (`crates/tea-client/tests/proxy_env.rs`).

**Weak checksums:** when every declared checksum uses a weak algorithm (`SHA-1`; `MD5` is flagged weak but never computed and is shorter than the schema's 40-character minimum), a full match is `IntegrityOutcome::Weak`, not `Verified`, and `Service::download_*` returns `ClientError::Integrity`. `ClientBuilder::allow_weak_checksums(true)` (CLI `--allow-weak-checksums`, core `ChecksumPolicy::with_allow_weak`) reports such a match as `Verified`; `ChecksumCheck::weak` stays `true` so callers can still see it. A weak checksum next to a strong one does not block `Verified`; a weak mismatch is always `Failed`.

`deflate` is decoded as the zlib wrapper defined for HTTP, not as raw DEFLATE. `gzip` is unchanged. Encoded input and decoded output are both limited by the artifact size cap. `HEAD` and `304` responses are not treated as bodies; `Content-Length` on those responses describes the selected representation.

## Integrity

Checksums are computed on the bytes after one content coding is removed. `gzip` uses the gzip wrapper. HTTP `deflate` uses the zlib wrapper. Unknown content codings are errors. The encoded body is not hashed and is not returned as the artifact. Decoding happens while the body is read. The encoded size is rejected before a chunk is inflated. The decoded buffer refuses to grow past the artifact cap inside the decoder write, so one compressed chunk cannot expand the buffer past that cap before the error is returned. A `deflate` body is returned only when the zlib member reaches stream end. A truncated member, including a missing Adler-32 trailer, is an error and is not returned as artifact bytes.

`IntegrityOutcome::Verified` is returned only when every declared checksum matched. An unsupported algorithm, a missing digest, or a mismatch is not success. `ETag` is not a checksum. A weak `ETag` is recorded as not strong. `Repr-Digest` is recorded and is not used as an artifact checksum.

## Limits

Defaults: discovery body 1 MiB, JSON body 8 MiB, artifact body 256 MiB, five redirects, two transport retries, four discovery attempts, and eight in-flight requests. Timeouts (request, download, idle and connect) are in [architecture.md](architecture.md#timeouts). Callers can change these limits. `max_in_flight` must be at least 1.

Retries apply to transport failures, timeouts, and HTTP 429 and 503 on `GET`; a `Retry-After` longer than `Limits::max_retry_after` fails at once with `ClientError::Throttled`. They do not apply to TEA error bodies. The discovery attempt bound is separate from the per-request retry bound.

## Dependency and supply-chain notes

The lockfile is committed. `deny.toml` allows common permissive licenses and denies unknown registries and yanked crates. CI runs `cargo deny`. CodeQL (`.github/workflows/codeql.yml`) scans the Rust code, the workflows and the Python release scripts on every pull request; findings appear under code scanning. Fuzz targets live under `fuzz/` and are not part of the library workspace.

`#![forbid(unsafe_code)]` is set on every library and on the CLI. The HTTP stack and hash crates may contain unsafe code in their own crates.

## Out of scope

This SDK does not evaluate trust anchors, Sigstore trust policy, or publisher identity beyond TLS to the host it was told to use. A verified checksum shows that the bytes match the digest the server declared for that format. It does not show that a publisher signed the bytes, and it does not show that a third party published them. The one signature check it offers is `KeyVerifier` (feature `signature-keys`, and `libtea download --signature-key`): raw Ed25519 or ECDSA P-256 detached signatures over the decoded bytes, under public keys the caller supplies. It never takes a key from the TEA server and trusts none by default, so a server cannot make its own signature verify. What `Verified` then shows is that someone holding one of those keys signed the bytes; who that is, is the caller's knowledge. OpenPGP, CMS and Sigstore signatures are detected and reported as unsupported.

## Assurance case

Why the security requirements above are met, in four parts. Each claim points
to the section that gives the detail and to the tests that check it.

**Threat model.** The attacker controls, or sits between the client and, any
server the client talks to: the TEI domain's well-known document, a discovered
API endpoint, an artifact or signature host, a redirect target. They want to
(1) obtain the API key or a bearer token, (2) make the client accept bytes that
do not match what was declared, (3) use the client to reach internal addresses
(SSRF), or (4) exhaust memory, disk or time. A local attacker who can write the
cache directory is covered under "Persistent cache". Publisher identity is out
of scope (see "Out of scope").

**Trust boundaries.** Listed under "Trust boundaries": the caller-chosen origin
is the only one that receives credentials by default; discovered endpoints,
artifact URLs and redirect targets are untrusted, and each hop is re-checked.

**Secure design principles applied.**

- *Secure defaults and no unsafe switches:* TLS 1.2+ with verification that
  cannot be turned off; HTTPS to public addresses only; weak-only checksums
  rejected; no signature key trusted by default. Each relaxation is one named,
  per-client opt-in (`allow_origin`, `allow_discovered_credentials`,
  `allow_weak_checksums`).
- *Least privilege:* the bearer token is attached only under the versioned API
  prefix of an authorized origin, never to artifact or signature hosts; the
  API key is sent only to `POST /token`.
- *Complete mediation:* every request and every redirect hop goes through
  `DestinationPolicy`, and DNS answers are filtered for the request in progress
  (the proxy exception is documented under "Transport").
- *Fail closed:* an unsupported algorithm, a missing digest, a truncated body,
  an unknown content coding or an unverifiable signature is never success.
- *Economy of mechanism:* `#![forbid(unsafe_code)]` on every crate; one HTTP
  stack (reqwest with rustls); limits on every body, redirect chain and retry.

**Common weaknesses countered.**

| Weakness | Countered by |
| --- | --- |
| CWE-918 SSRF | `DestinationPolicy` and DNS filtering ("Transport") |
| CWE-522 / CWE-532 credentials sent or logged | token scoping, redacted `Debug` and logs ("Authentication") |
| CWE-295 certificate validation | rustls with webpki roots, no off switch ("Transport") |
| CWE-328 / CWE-354 weak or missing integrity check | all declared checksums must match; SHA-1-only rejected ("Integrity", "Transport") |
| CWE-400 / CWE-409 resource exhaustion, decompression bombs | size caps checked inside the decoder, timeouts, retry bounds ("Integrity", "Limits") |
| CWE-22 / CWE-59 path traversal and symlinks in CLI output | file names never come from server data; `--output` refuses an existing path unless `--force`, and `--output` and `--signature-output` refuse a symbolic link as the final component (parent directories are not checked; `libtea download --help`) |
| CWE-119 memory safety | Rust, `#![forbid(unsafe_code)]` |
| CWE-1357 / CWE-829 untrusted dependencies and build tools | `cargo deny`, `cargo audit`, pinned actions and tools, SBOM and provenance per release ("Dependency and supply-chain notes", `docs/release-pins.md`) |

How this is checked: unit and integration tests named in each section,
13 fuzz targets run in CI, CodeQL, clippy with `-D warnings`, and at least 90%
line coverage (`CONTRIBUTING.md`).

## Persistent cache

The opt-in response cache (`docs/cache.md`) is not a trust boundary: its entries
are checked for corruption, not authenticity, and anyone who can write the
directory can feed the client fake metadata. The directory is private by
construction, artifact bytes are re-checked against declared checksums on every
use, and credentials and secrets are never stored.
