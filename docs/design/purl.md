# PURL canonicalization (ECMA-427): design

**Status:** stage 1 shipped in 0.1.7 (issue #10): `libtea_core::purl`. Stage 2
(#60): `pypi` shipped in 0.1.9 ("Stage 2: pypi"), and the rules every type
definition gives as data follow in the next library release ("Stage 2: rules
from the type definitions", at the end).
The proposal below is kept as written; "What was built" at the end says where
the implementation differs and which statements in it turned out to be wrong.

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

## What was built

Decisions taken: own implementation (no new dependency), generic rules only
(stage 2 later, type by type), additive and Experimental.

- `libtea_core::purl::{Purl, CanonicalPurl, PurlRules, PurlError,
  same_package_url, MAX_PURL_BYTES}`, also at the crate root. `Purl` has
  accessors for every component (`package_type`, `namespace`, `name`,
  `version`, `qualifiers`/`qualifier`, `subpath`) besides `as_str` and
  `canonical`. `Tei::purl()` and `Identifier::purl()` as proposed; a `purl`
  TEI whose identifier does not decode is `PurlError::TeiIdentifier`.
- No builder from components: the suite's 177 `build` cases are skipped.
- Test data: the purl-spec `v1.1.1` test suite, vendored under `spec/purl/`
  with SHA-256 digests (`spec/purl/PINNED.md`, checked by a test). 374 parse
  and round-trip cases pass. 41 need a type-specific rule or are contradicted
  elsewhere in the suite; each is listed in
  `crates/tea-core/tests/purl_spec_suite.rs` with its reason, and the test
  fails if one starts passing.
- The 4 KiB limit also applies to the canonical form, which can be three
  times longer than the input (each escaped octet is `%XX`). Without that,
  a valid input's canonical form could fail to parse; the `purl_parse` fuzz
  target found this before the change.
- An unencoded `@` that starts a namespace segment (`pkg:npm/@babel/core`)
  is not a version separator: the version is split from the last path
  segment only. The suite has this case; how-to-parse, read literally, would
  take `babel/core` as the version.
- In 0.1.8 (part of #60): `Service::discover_purl` (`libtea-client`) and
  `libtea discover --purl` send a parsed `Purl` to `/discovery` as given
  (`Purl::as_str`), not canonicalized.

**Corrections to the proposal above**, from ECMA-427 clause 5 and the type
definitions at the pinned commit:

- The type allows ASCII letters, digits, `.` and `-` and must start with a
  letter. `+` is not allowed (the proposal listed `[a-z0-9.+-]`).
- Empty, `.` and `..` subpath segments are discarded when parsing, not
  rejected.
- Qualifier keys must start with a letter and must be unique; a repeated key
  is an error.
- `:` is never percent-encoded; every octet other than ASCII alphanumerics,
  `.-_~` and `:` is, with uppercase hex.
- npm namespaces and names are **case-sensitive** (`npm-definition.json`);
  the stage 2 list above was wrong to lowercase them. The type definitions
  carry `case_sensitive` flags per component, which stage 2 can use.

**Where the suite disagrees with itself or with how-to-parse.** For
`pkg:gem/jruby-launcher@1.1.2?Platform=java` the `validate` case expects
`platform=java` while the `parse` case for the same input expects a failure;
`rpm` has the same pair, and maven's `repositorY_url` is expected to parse.
This implementation lowercases keys, as how-to-parse says, and lists the two
`parse` cases. Not reported upstream yet.

## Stage 2: pypi

From the purl-spec `pypi` type definition at the pinned commit
(`types/pypi-definition.json`, not vendored; the suite's `pypi-test.json` is):
the name is case-insensitive, with the normalization rules "The name shall be
lowercase" and "Replace runs of consecutive dash '-', underscore '_', or dot
'.' characters with a single dash '-'" (PEP 503), and the version is marked
case-insensitive with no normalization rule.

- `Purl::canonical` applies both to `pypi` PURLs: the name is lowercased and
  its runs collapsed, the version is lowercased. Only ASCII letters are
  lowercased, because valid PyPI names and versions are ASCII. Lowercasing the
  version follows `case_sensitive: false`; the suite has no case for it. No
  PEP 440 normalization of the version (`1.0-1` vs `1.0.post1`): the
  definition does not ask for it.
- `CanonicalPurl::rules()` is the new `PurlRules::Pypi` for those, and
  `Generic` for every other type. `Purl`'s accessors still return what was
  written (`name()` of `pkg:pypi/Django_Allauth` is `Django_Allauth`).
- The definition prohibits a namespace. A `pypi` PURL with one still parses:
  rejecting components by type is a separate rule, not applied for any type.
- The suite's `parse` cases are now compared on the components of the
  canonical form, since the suite expects normalized components. 380 cases
  pass (374 before) and 35 are listed (41 before). Six `pypi` cases pass now.
  The seventh stays listed: for `pkg:pypi/My.PyPI_.--._Package@1.11.1.dev1`
  the expected `subpath` is the sentence "Applied normalization_rules to
  convert input string to valid form.", which no parser can produce. Not
  reported upstream yet.
- **Behaviour change, Experimental API:** two `pypi` PURLs that differ only in
  name case or separators, or in version case, now compare equal under
  `same_package_url`, and their canonical text changes.

## Stage 2: rules from the type definitions

The 42 purl-spec type definitions at the pinned commit are now vendored
(`spec/purl/types/*-definition.json`, digests in `spec/purl/PINNED.md`). The
module applies what they give as **data**, and nothing they give only in prose:

- **Case.** Every component (namespace, name, version, subpath) marked
  `"case_sensitive": false` is lowercased by `Purl::canonical`, for 21 types
  (`alpm`, `apk`, `bitbucket`, `bitnami`, `brew`, `chrome-extension`,
  `composer`, `deb`, `github`, `hex`, `huggingface` (version only),
  `luarocks`, `oci`, `otp` (name and subpath), `pub`, `pypi`, `qpkg`, `rpm`
  (namespace only), `vscode-extension`, `yocto`). `CanonicalPurl::rules()` is
  the new `PurlRules::CaseInsensitive` for them, `Pypi` for `pypi`, and
  `Generic` for types whose definition marks nothing case-insensitive (`npm`,
  `maven`, `cargo`, `golang`, `swift`, ...) and for types without a
  definition. Only ASCII letters are lowercased. This replaces the hand-written
  candidate list in issue #60: `golang` and `git` are case-sensitive in their
  definitions, so they are not folded.
- **Type checks.** `Purl::check_type()` returns a `PurlTypeError` for a
  missing required namespace or a namespace the type prohibits (the
  `requirement` of `namespace_definition`), a missing required qualifier
  (`julia`'s `uuid`, `swid`'s `tag_id`), and characters outside the
  `permitted_characters` patterns. There are three such patterns, written out
  by hand because the module has no regular-expression engine:
  `chrome-extension` names `^[a-p]{32}$` (after case folding) and versions
  `^\d+(\.\d+){0,3}$`, and `pub` names `^[a-z0-9_]`, which as written
  constrains only the first character. `Purl::parse` does not call it: a PURL
  its type does not allow still parses, and the caller decides.
- **The table is checked against the data.** `definitions_match_the_module`
  in `crates/tea-core/tests/purl_spec_suite.rs` builds a PURL of every
  defined type and checks which components `canonical` lowercases, which
  `rules()` it reports, and what `check_type` says with and without a
  namespace and each required qualifier. It also fails if a definition's
  `permitted_characters` change, so a re-pin cannot silently disagree with the
  code.
- **Not applied (prose only):** `pub`'s "replace non-[a-z] letters, non-[0-9]
  digits with underscore", `hackage`'s "kebab-case", `alpm`'s `vercmp`
  version normalization, `cpan`'s uppercase author and "no `::` in a
  distribution name", `mlflow`'s "lowercased for Databricks", and `git`'s
  host-then-path split. Each would need its own reading of the text.
- **Suite:** 402 cases pass (380 before). 12 entries are listed (35 before),
  covering 13 cases, because the suite has one `swift` case twice. The 177
  `build` cases are skipped as before. Of the 12 entries, 7 need a prose rule:
  3 `cpan`, 2 `git` and the 2 `mlflow` cases that need the Databricks rule.
  The other 5 are upstream problems:
  - the `gem` and `rpm` `parse` cases, which contradict their `validate` pair;
  - the `pypi` subpath sentence;
  - the `git` `validate` case, which lowercases a namespace and name that its
    definition marks case-sensitive;
  - the `swift` case `pkg:swift/github.com/Alamofire/@5.4.3`, which expects no
    name, where the generic rules drop the trailing `/` and leave `Alamofire`.

  None are reported upstream yet.
- **Behaviour change, Experimental API:** PURLs of the 21 types that differ
  only in the case of a case-insensitive component now compare equal under
  `same_package_url`, and their canonical text changes
  (`pkg:github/Package-url/purl-Spec` is `pkg:github/package-url/purl-spec`).
  Nothing sent, stored, locked or printed changes. `PurlTypeError`,
  `Purl::check_type` and `PurlRules::CaseInsensitive` are additive.
- **Not done, on purpose:** a builder from components (the suite's `build`
  cases). Nothing in the SDK or the CLI needs one.
