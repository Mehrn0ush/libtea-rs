# Changelog

All notable changes to this workspace are documented here. The publishable crates are `cdx-tea-core` and `cdx-tea-client`. Versions below refer to those crate versions unless noted.

## 0.1.0 — unreleased

### Added

- Consumer TEA 1.0.0 client surface (`cdx-tea-client::Service`) against pinned OpenAPI commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`.
- TEI parsing, discovery, destination policy, checksum verification on decoded artifact bytes, API-key → bearer exchange.
- CLI (`cdx-tea-cli`, unpublished): `spec`, `parse-tei`, `conformance`, `discover`, `product-release`, `collections`, `download`.
- Credential environment variables `TEA_API_KEY_ID` / `TEA_API_KEY_SECRET` (preferred over `--api-key-secret`).
- Concurrent token-refresh serialization via a client-wide async mutex (waiters may reuse a cached bearer; not a guarantee of exactly one exchange); see `docs/security.md`.
- Offline conformance smoke crate (`cdx-tea-conformance`, unpublished).

### Security

- HTTPS / public-destination defaults; origin-scoped exceptions; redirect credential scoping; transport URL redaction in errors.
- Checksum `Verified` only when every declared algorithm is implemented and matches; never implied authenticity.

### Known limitations

- No cryptographic signature verification, publisher authenticity verification, or Sigstore trust-policy evaluation. Signature bytes may be downloaded; they are not interpreted.
- Live interoperability evidence is Oolong (local) and Vulnetix (public read-only); not universal compatibility.
- CLI binary name remains `tea` (reconsider before publishing the CLI).
