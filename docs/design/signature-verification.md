# Signature verification — design (libtea-rs)

**Status for 0.1.0:** design + extension point only.  
**Ship rule:** no built-in verifier returns “Verified”.

Checksum `IntegrityOutcome::Verified` means declared digests matched **decoded**
artifact bytes. It does **not** mean a publisher signed the bytes.

## Threat model (consumer)

| Asset | Threat | Checksums alone | Crypto signature (future) |
| --- | --- | --- | --- |
| Artifact bytes in transit/at rest as published by the TEA host | Bit flips / wrong object | Detected if digests are present and match | Same host can publish matching fake digests |
| Publisher identity | Impersonation by TEA host or CDN | **Not** addressed | Requires trust anchors **not** solely from that host |
| Supply-chain attestation (Sigstore, etc.) | Forged provenance | **Not** addressed | Needs log/policy evaluation |

Verifying a signature with keys fetched only from the same untrusted host that
served the artifact would be misleading. Trust anchors must be caller-supplied
or obtained from an independent trust root.

## What “Verified” would mean (future)

Only after **all** of:

1. Signature bytes parse under a supported format.
2. Cryptographic validation succeeds.
3. Signer identity binds to a **caller-configured** trust policy.
4. Optional: transparency-log / freshness checks for Sigstore-like formats.

Never map “bytes downloaded” or “checksum matched” to cryptographic Verified.

## Trust-anchor sourcing

- TEA OpenAPI at the pinned commit does **not** define `/trust-anchors/`.
  Informative `signatures/signature.md` is outside the standard and is not
  implemented as an API surface here.
- 0.1.0: no automatic trust-anchor fetch.
- Future: PEM/keyring trait, and optionally Sigstore TUF roots — caller opts in.

## Formats to consider (roadmap)

| Format | Milestone | Notes |
| --- | --- | --- |
| Detached raw signature over digest with caller keys | 0.2 (candidate) | Smallest honest MVP |
| CMS / OpenPGP detached | 0.2–0.3 | Only if publishers converge |
| Sigstore bundle | 0.3 | Separate feature; log policy required |

## Failure modes

`SignatureVerifyOutcome`: `Verified` | `Invalid` | `Unsupported` |
`NoTrustAnchor` | `MissingSignature` (see `libtea_client::signature`).

## Extension point (0.1.0)

`libtea_client::SignatureVerifier` receives artifact bytes + signature bytes and
returns a typed outcome. The HTTP client does **not** call it automatically.
Test doubles must not return `Verified` unless simulating a caller policy in
tests.

## Staged roadmap

- **0.1.0:** download signature bytes; checksum integrity; trait + design doc.
- **0.2.0:** optional verifier crate/feature with caller-supplied keys; no default
  trust.
- **0.3.0:** optional Sigstore path behind an explicit feature and documented
  policy inputs.

## Stage 1: open questions and a proposed first slice (issue #12)

*Added 2026-11-11 for review. Not implemented.*

### What the pinned specification says

`/artifact/{uuid}/{artifactVersion}/signature/download` (`openapi.yaml`):
signatures are **per format** (the `mediaType` parameter selects the artifact
format, not the signature format); a format may point to an external
`signatureUrl` instead; the specification "makes no assumption about the
signature technology in use"; and "A client that cannot determine how to verify
what it receives should treat the signature as unusable rather than as
invalid." That last sentence is `SignatureVerifyOutcome::Unsupported`, not
`Invalid`.

### Open questions this document leaves

1. **Which formats do publishers actually use?** No public TEA server in
   `docs/interop/` serves signatures today. Without one, any format choice is a
   guess. The milestone table above is a guess.
2. **What is the signed message?** The artifact bytes, their digest, or a
   statement about them (as in a Sigstore bundle or an in-toto attestation)?
   Each format answers differently; the trait takes the bytes, which is right
   for detached signatures and wrong for bundles that carry their own subject.
3. **Where does the trust policy live?** The trait has no policy parameter. A
   caller-configured verifier holds it, which is fine, but nothing records
   *which* policy produced `Verified`, so an outcome cannot be audited later.
4. **How does a result reach the CLI and lock file?** `sync` records checksums;
   a signature outcome has no field in `lock-v1` and no `--json` shape.
5. **External `signatureUrl`.** The destination policy (`DestinationPolicy`)
   applies to it like any URL, but should a signature from another origin than
   the artifact be accepted at all? Unanswered.
6. **Should the trait be async or receive the artifact as a stream?** Large
   artifacts are streamed to disk today (`FileDownload`); `verify(&[u8], ..)`
   forces them into memory.

### Proposed first slice

Small, honest, and useful without choosing a cryptographic library:

- `libtea_client::signature::SignatureFormat` (`#[non_exhaustive]`):
  `OpenPgpArmored`, `OpenPgpBinary`, `Cms` (DER `SignedData`), `SigstoreBundle`
  (JSON with a `mediaType` starting `application/vnd.dev.sigstore.bundle`),
  `Unknown`. Detection is by content (armor header, packet tag, DER OID
  `1.2.840.113549.1.7.2`, JSON `mediaType`) with the server's `Content-Type` as
  a hint only; a size cap (for example 1 MiB) before any parsing.
- `pub fn detect_signature_format(bytes: &[u8], content_type: Option<&str>) ->
  SignatureFormat`: pure, no allocation beyond the JSON check, fuzzed.
- A built-in `FormatOnlyVerifier` that implements `SignatureVerifier` and
  always returns `Unsupported` (or `MissingSignature` for empty input), so the
  CLI can report "signature present, format X, not verified" without claiming
  anything.
- CLI: `libtea download --signature` prints the detected format and
  `not verified` in text and `--json`; no exit-code change.
- Tests: one fixture per format (generated, not from a real publisher), empty
  and oversized input, a JSON document that is not a bundle, and a property
  test that detection never panics. New fuzz target `signature_format`.

What it deliberately does not do: verify anything, fetch keys, or return
`Verified`. The ship rule at the top of this document stands.

### Decisions for you

1. Whether this slice is worth shipping before any server serves signatures,
   or whether to wait for question 1. Recommendation: ship the detection only
   if a server appears; otherwise park issue #12.
2. Whether to change the trait (questions 2, 3 and 6) before anyone implements
   it. It is marked Experimental, so a 0.x minor release may change it; doing
   so before a second implementation exists is cheapest.
