# Roadmap

What the project intends to do next. Nothing here has a date. Items move to
[CHANGELOG.md](CHANGELOG.md) when they ship. Each item is discussed in its issue
or design note before it is built.

## Next

- **Re-pin to TEA 1.0.** The libraries are written against TEA commit
  `962a0228` (OpenAPI 1.0.0, specification still beta). When the TEA working
  group publishes 1.0, the pin moves to it, following `spec/RE-PINNING.md`. A
  lock file written before the re-pin stays valid: since `libtea-core` 0.2.1 a
  different `specCommit` is a note, not a change.
- **Container image** (issue #17). An image built from the release archives
  after they pass the `SHA256SUMS` and attestation checks, with its own
  build-provenance attestation. A design note comes first, because publishing
  an image needs a new permission in the release workflow. The Homebrew part of
  #17 shipped in 0.2.2.

## Later

- **More version schemes** for `vers` ranges and CLE evaluation, beyond SemVer
  and PEP 440, one scheme at a time (`docs/design/cle.md`).
- **More Package URL type rules**, type by type (`docs/design/purl.md`).
- **Sigstore signature verification**, behind an explicit feature, with the
  trust policy supplied by the caller (`docs/design/signature-verification.md`).
  Raw Ed25519 and ECDSA P-256 under caller-supplied keys shipped in 0.2.x.

## Stability

Items marked Experimental may change in a minor release until 0.9. A 1.0
release of the libraries waits for TEA 1.0 and for the Experimental items to
settle. Supported versions and the support period are in
[SECURITY.md](SECURITY.md).

## Out of scope

- The TEA publisher API: libtea-rs is a consumer client.
- Disabling TLS certificate verification, trusting signature keys by default,
  or fetching keys from a TEA server. These are security properties, not
  missing features ([docs/security.md](docs/security.md)).

OAuth flows other than API-key `client_credentials`, HTTPS SVCB lookup, and
OpenPGP or CMS verification are not planned now. An issue with a concrete
server that needs one of them can change that.
