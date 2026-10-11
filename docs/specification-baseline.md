# Specification baseline

This SDK implements the consumer side of the Transparency Exchange API from one pinned revision. It does not follow the specification forward automatically.

| Field | Value |
| --- | --- |
| Repository | https://github.com/CycloneDX/transparency-exchange-api |
| Branch | `main` |
| Commit | `962a0228af1bfcc1232ac3f79239317ff33fdb04` |
| Commit date | 2026-10-08T06:28:26Z |
| Retrieval date | 2026-10-10 |
| Commit subject | Merge pull request #410 from Mehrn0ush/docs/readme-bom-exchange-link |
| Specification version (`info.version`) | `1.0.0` |
| OpenAPI version | `3.1.1` |
| ECMA context repository | https://github.com/Ecma-TC54/ECMA-xxx-TEA |

The OpenAPI document at that commit is `spec/openapi.yaml`. Its `info.version` is `1.0.0`. The repository README still says "Beta 2". This SDK treats the OpenAPI document and the normative discovery, authentication, collection, and TEI registry files at this commit as the contract, and records the README status line as stale. See [specification-ambiguities.md](specification-ambiguities.md).

## Normative source files

- `spec/openapi.yaml` — consumer operations, schemas, and HTTP response metadata
- `discovery/readme.md` and `ecma-doc/discovery.md` — TEI syntax, well-known retrieval, version negotiation, failover, and authentication interaction (these two files match at this commit)
- `discovery/tea-well-known.schema.json` — `.well-known/tea` schema
- `auth/readme.md` — API key exchange and bearer usage
- `tei-types/*.md` — TEI type registry
- `tea-collection/tea-collection.md` — collection and artifact byte rules
- `doc/tea-uuid-scope.md` — UUID scope
- `api-flow/consumer.md` — informative consumer flows

## Informative documents

- `README.md`
- `doc/tea-requirements.md`
- `doc/tea-usecases.md`
- `doc/tea-implementations.md`
- `signatures/signature.md` — explicitly not part of the TEA standard
- `spec/publisher/openapi.json` — publisher API, out of consumer scope

## Known implementation limitations

- HTTPS SVCB records are not queried. The discovery document allows them when a client is known to support them. This client uses `A`/`AAAA` resolution through the system resolver and then the HTTPS URL from the TEI domain.
- There is no persistent HTTP cache. Callers can send `If-None-Match`. `Cache-Control` is returned and not stored.
- Detached signature bytes are downloaded. Signature technology is not interpreted. `signatures/signature.md` describes a `/trust-anchors/` resource that is not in the OpenAPI document, so it is not implemented.
- Optional OAuth profiles other than the API-key `client_credentials` baseline (SAML, JWT bearer, token exchange, mutual TLS) are not implemented.
- CLE is retrieved as the TEA projection. This client does not fetch or validate a full ECMA-428 document.
- The SDK is not `no_std` and is not a TEA server.
- List filters accept `idType` alone and both `idType` and `idValue`. `idValue` without `idType` is rejected before the request. The pinned text does not define that value-only case, and the current error string (`idType and idValue must be provided together`) is broader than the check.
- The CLI covers spec identity, TEI parse, offline conformance, discovery, one product-release read, and artifact download. It does not offer list or identifier-filter commands.

## Unresolved specification ambiguities

See [specification-ambiguities.md](specification-ambiguities.md).
