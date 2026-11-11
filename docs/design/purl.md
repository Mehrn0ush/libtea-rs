# PURL canonicalization (ECMA-427): design

**Status:** proposal for issue #10. Nothing here is implemented. Review before
any code is written.

## What the pinned specification allows

The pinned TEA commit (`962a0228`) constrains this more than the issue title
suggests:

- `discovery/readme.md`: "A client shall use a TEI exactly as received and shall
  not rewrite it." A `tei://<domain>/purl/<base64url>` identifier is therefore
  never re-encoded, even if the PURL inside it is not canonical.
- `openapi.yaml`, `/discovery`: the `purl` query parameter is "the logical PURL
  string" and "Servers shall interpret the value after one query-decoding step".
  Nothing asks the client to canonicalize what it sends; a client that did would
  ask the server a different question than the caller asked.
- Lock files (`docs/lock-file.md`) record what the server returned. Rewriting
  identifiers there would make `libtea sync --check` report differences the
  server never made.

So canonicalization in this SDK is a **comparison aid only**. It never changes
a value that is sent, stored, locked or printed as the server's.

## Where comparing PURLs matters today

- `identifiers[]` with `idType: PURL` on products, product releases and
  components: a caller asking "is this the package I think it is" compares a
  PURL they hold with one the server returned.
- Discovery by PURL returns product release UUIDs; a caller that discovered by
  one spelling may want to check that a returned identifier names the same
  package.

The SDK currently passes PURLs through unchanged (`tei.rs` only checks that a
`purl` TEI decodes from unpadded Base64URL to text starting with `pkg:`).

## Proposed API (libtea-core, additive)

```rust
/// A parsed Package URL. Holds the original text; never rewrites it.
pub struct Purl { /* original: String, parts */ }

impl Purl {
    pub fn parse(text: &str) -> Result<Self, PurlError>;
    pub fn as_str(&self) -> &str;              // exactly the input
    pub fn canonical(&self) -> CanonicalPurl;  // computed, separate type
    pub fn package_type(&self) -> &str;        // lowercased
}

/// A PURL in ECMA-427 canonical form. Comparable, hashable, displayable.
/// Not convertible back into a `Purl` without parsing, so it cannot be sent by
/// mistake where the original was meant.
pub struct CanonicalPurl(String);

impl CanonicalPurl {
    /// Which rules were applied: `Generic` or `TypeSpecific(type)`.
    pub fn rules(&self) -> PurlRules;
}

/// Same package, version, qualifiers and subpath after canonicalization.
pub fn same_package_url(a: &Purl, b: &Purl) -> bool;
```

`Tei` gets `purl(&self) -> Option<Result<Purl, PurlError>>` for `purl`-type
TEIs, decoding the Base64URL identifier. `Identifier` (the model type) gets the
same for `idType: PURL`. Both are additive; `#[non_exhaustive]` on the new types.

## Canonicalization rules, staged

The ECMA-427 text is not vendored in this repository. The rules below are from
the Package URL specification as I know it and must be checked against the
standard (and its test-suite file) before implementation; that check is the
first task of the implementation PR.

**Stage 1 (generic, all types)** from ECMA-427:
- scheme `pkg` lowercase; no `//` after the scheme;
- `type` lowercase;
- percent-encoding normalized: decode, then re-encode exactly the characters the
  standard requires, with uppercase hex;
- qualifier keys lowercase, sorted; empty qualifier values dropped;
- subpath: leading and trailing `/` removed, `.` and `..` segments rejected,
  empty segments removed;
- `version` and `name` otherwise unchanged.

**Stage 2 (type-specific)**, only for types whose rules are stable in the
standard, each with its own tests: `pypi` (name lowercase, `_` → `-`), `npm`
(namespace and name lowercase), `github`/`bitbucket` (lowercase namespace and
name), `maven` (none: case-sensitive), `deb`, `rpm`, `golang`. Unknown types get
generic rules only, and `rules()` says so, so a caller knows the comparison may
miss equivalences a type-aware tool would find.

**Rejected inputs:** missing `pkg:`, missing type or name, a type with
characters outside `[a-z0-9.+-]` after lowercasing, invalid percent-encoding,
`..` in the subpath, and input over 4 KiB. Rejection is a `PurlError`, never a
panic.

## Decisions for you

1. **Own implementation or the `packageurl` crate.** Not evaluated yet: its
   coverage of the type-specific rules, its MSRV and its maintenance need to be
   checked. Either way it would be a new dependency of `libtea-core`.
   Recommendation: own implementation of stage 1, which is small; evaluate the
   crate for stage 2.
2. **Stage 2 in the first PR or later.** Recommendation: later, type by type.
3. **Version bump.** All additive: 0.1.x patch release under the Cargo 0.x
   rules, same as `libtea_client::core` in 0.1.5.

## Tests and fuzzing

- Table tests from the Package URL specification's test-suite data (vendored
  under `spec/` with its commit and SHA-256, like the OpenAPI file), each case asserting the canonical
  form and that `as_str()` is unchanged.
- Property tests: `canonical(parse(canonical(x))) == canonical(x)` (idempotent);
  `same_package_url` is reflexive and symmetric.
- New fuzz target `purl_parse`: arbitrary bytes into `Purl::parse` and, on
  success, `canonical()`; asserts no panic, idempotence, and that canonical
  output parses. Seed corpus from the test-suite file.
- A test that `Tei::parse` of a `purl` TEI followed by `as_str()` returns the
  input byte for byte, whatever the PURL's canonical form.
