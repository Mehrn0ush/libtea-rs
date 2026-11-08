# Signature verification — technical design (deferred)

**Decision for 0.1.0:** deferred. Do not ship cryptographic signature or Sigstore verification in `libtea-core` / `libtea-client`.

The SDK downloads detached signature bytes and records `SignatureFetch::{Present,NotPublished,NotModified}`. Checksum verification of artifact content remains separate and does **not** imply publisher authenticity.

## Why defer

| Criterion | Finding against pinned TEA 1.0.0 (`962a0228…`) |
| --- | --- |
| Signature format prescribed? | No normative single format. Formats expose optional `signatureUrl` / signature download paths; content type is not a TEA-mandated crypto profile. |
| Trust policy defined by TEA? | No. TEA does not define trusted roots, expected signer identity, or certificate policy for consumers. |
| Sigstore normative? | No. A server may publish a Sigstore provenance *artifact* (as Vulnetix does). That is artifact distribution, not a TEA requirement to verify bundles. |
| Safe default without trust config? | No. Verifying with keys fetched from the same untrusted host that served the artifact would be misleading. |
| Optional crate without destabilizing 0.1.0? | Yes, but only after formats and trust APIs are decided. |

Implementing a partial verifier that returns “verified” without an explicit trust policy would be worse than no verifier.

## Layering (future)

Keep these concerns separate:

1. Artifact retrieval (`libtea-client`)
2. Content integrity / checksums (`libtea-core`, decoded bytes)
3. Cryptographic signature verification (proposed `libtea-verify` or feature-gated module)
4. Signer identity binding
5. Trust-policy evaluation (caller-supplied roots / identities)
6. Provenance / transparency-log checks (Sigstore-specific)

HTTP transport must not decide trust.

## Proposed future API sketch

```text
TrustPolicy { trusted_keys | trusted_cas | sigstore_roots, expected_identity? }
VerificationRequest { artifact_bytes, signature_bytes, media_hint?, digest_binding? }
VerificationOutcome:
  ValidAndTrusted
  ValidButUntrusted
  Invalid
  UnsupportedFormat
  MissingSignature
  MissingTrustPolicy
  NotPerformed
```

Never map “ValidButUntrusted” or “NotPerformed” to a success named `Verified`.

## Candidate dependencies (evaluation only)

- `sigstore` / `sigstore-verification` crates — only if MSRV, license (`deny.toml`), and API stability fit.
- OpenPGP (`sequoia`) — only if TEA publishers converge on that detached format.
- Raw signature over digest with caller-supplied public keys — smallest honest MVP.

Do not vendor custom crypto primitives.

## Test vectors required before implementation

- Known-good signature + trusted key
- Known-good signature + untrusted key
- Modified artifact / modified signature
- Wrong signer identity
- Unsupported algorithm / malformed signature
- Missing trust roots
- Expired certificates (when PKIX is in scope)
- Incorrect digest binding
- Sigstore bundle + transparency requirements (if that path is supported)

## Remaining decisions

1. Which signature media types are in scope for v1 of `libtea-verify`?
2. Is Sigstore a separate feature from detached CMS/PGP?
3. How are trust anchors supplied (PEM files, keyring trait, Sigstore TUF)?
4. Does verification bind to TEA checksums, `Repr-Digest`, or both?

Until those are answered, leave verification out of the Consumer SDK release.
