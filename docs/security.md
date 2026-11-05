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

**Token-refresh serialization (0.1.0 decision):** refresh uses one **client-wide** `tokio::sync::Mutex`, not a per-origin map of locks. Rationale: a `Client` carries a single API key; the bearer cache is already origin-keyed. Behavior:

1. A challenge that should refresh acquires the mutex before `POST /token`.
2. After waiting, if the cache already holds a usable bearer different from the value that received `401` (or any bearer when none was presented), that token is reused and no additional exchange runs.
3. Otherwise the client exchanges the API key once under the lock and stores the result for that origin.

This often collapses overlapping challenges into one exchange, but it is **not** a guarantee of exactly one `POST /token` for every concurrent set: a failed exchange, an expired or discarded cache entry, or challenges against another origin can each produce another request. The client-wide lock can also temporarily serialize refreshes for unrelated origins. Ordinary resource GETs are not serialized by this lock. Per-origin refresh locks were considered and deferred. Regression under a forced two-challenge gate: `concurrent_challenges_share_one_token_exchange`. The token cache key remains the origin (scheme, host, and port). Two version prefixes on that origin share the cached token, and the token is attached only when the request URL is under the prefix for that call.

### CLI credentials

Prefer `TEA_API_KEY_ID` and `TEA_API_KEY_SECRET`. A complete `--api-key-id` / `--api-key-secret` pair still works for compatibility, but placing the secret on the command line exposes it to process listings and shell history. Partial pairs and mixing a CLI id with an environment secret (or the reverse) are rejected. The CLI never prints the secret. Unit tests that temporarily clear those variables restore the previous process environment on exit so `cargo test` does not erase a developer's configured keys.

## Transport

reqwest is built with rustls. Certificate verification uses the webpki roots and any extra PEM roots the caller adds. There is no option to disable verification. The minimum TLS version is 1.2.

The default `DestinationPolicy` allows HTTPS to public addresses. It refuses HTTP, loopback, link-local, unique-local, private, CGNAT, documentation, benchmarking (`198.18.0.0/15`), `0.0.0.0/8`, and cloud-metadata addresses. Hostnames `localhost`, names under `.local` and `.localhost`, and the Google metadata hostnames are refused.

An exception from `DestinationPolicy::allow_origin` is one origin: scheme, hostname, and effective port. `https://example.test` and `https://example.test:443` are the same origin. `http://example.test`, `https://example.test:8443`, and a different hostname are not. Matching an origin does not authorize every address that hostname can resolve to on another scheme or port. A different hostname that resolves to the same address is still refused unless that second origin is excepted on its own.

DNS answers are filtered for the URL of the request in progress. A non-public answer is kept only when that exact origin is excepted. If the resolver is not running inside a request, only public addresses are kept. IPv4-mapped IPv6, 6to4, and NAT64 (`64:ff9b::/96`) are classified by the embedded IPv4 address. The filtered addresses are what the HTTP client connects to: reqwest does not look the name up again for that connection. A literal IP skips the resolver and is classified by `check_url` before the request is sent. This has been checked from the reqwest and hyper connection code. It has not been exercised with a hostile DNS server that changes its answer between lookups.

`deflate` is decoded as the zlib wrapper defined for HTTP, not as raw DEFLATE. `gzip` is unchanged. Encoded input and decoded output are both limited by the artifact size cap. `HEAD` and `304` responses are not treated as bodies; `Content-Length` on those responses describes the selected representation.

## Integrity

Checksums are computed on the bytes after one content coding is removed. `gzip` uses the gzip wrapper. HTTP `deflate` uses the zlib wrapper. Unknown content codings are errors. The encoded body is not hashed and is not returned as the artifact. Decoding happens while the body is read. The encoded size is rejected before a chunk is inflated. The decoded buffer refuses to grow past the artifact cap inside the decoder write, so one compressed chunk cannot expand the buffer past that cap before the error is returned. A `deflate` body is returned only when the zlib member reaches stream end. A truncated member, including a missing Adler-32 trailer, is an error and is not returned as artifact bytes.

`IntegrityOutcome::Verified` is returned only when every declared checksum matched. An unsupported algorithm, a missing digest, or a mismatch is not success. `ETag` is not a checksum. A weak `ETag` is recorded as not strong. `Repr-Digest` is recorded and is not used as an artifact checksum.

## Limits

Defaults: discovery body 1 MiB, JSON body 8 MiB, artifact body 256 MiB, five redirects, 30 second request timeout, 10 second connect timeout, two transport retries, four discovery attempts, and eight in-flight requests. Callers can lower these. `max_in_flight` must be at least 1.

Retries apply to transport failures, timeouts, and HTTP 503 on `GET`. They do not apply to TEA error bodies. The discovery attempt bound is separate from the per-request retry bound.

## Dependency and supply-chain notes

The lockfile is committed. `deny.toml` allows common permissive licenses and denies unknown registries and yanked crates. CI runs `cargo deny`. Fuzz targets live under `fuzz/` and are not part of the library workspace.

`#![forbid(unsafe_code)]` is set on every library and on the CLI. The HTTP stack and hash crates may contain unsafe code in their own crates.

## Out of scope

This SDK does not verify detached signature technology, trust anchors, Sigstore trust policy, or publisher identity beyond TLS to the host it was told to use. A verified checksum shows that the bytes match the digest the server declared for that format. It does not show that a publisher signed the bytes, and it does not show that a third party published them. Callers who need that assurance have to verify signatures themselves. The client can download signature bytes and does not interpret them.
