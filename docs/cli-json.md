# CLI `--json` output

Every command accepts `--json`. Output goes to **stdout**; on failure stdout is
empty and one error object goes to **stderr** (see the end). Field names below
are asserted by `crates/tea-cli/tests/json_and_exit_codes.rs`; renaming one
fails a test. Fields are snake_case. New fields may be added in a minor release;
consumers should ignore unknown fields.

Newer objects (`sync`, `diff`, `download` status, `cache`, `cle`, `inspect`) carry `schema_version`
(currently `1`). The first group of outputs (`spec`, `parse-tei`, `discover`,
`product-release`, `collections`, `conformance`) has no version field. All of
this is the contract since 0.1.0; it did not change through 0.1.6 (the CLI's
sources are identical between the `v0.1.0` and `v0.1.6` tags). In 0.1.7,
`download --json` gained an optional `signature` member (below); nothing
was renamed or removed. After 0.1.7, `discover --purl` (same array as
`discover <tei>`), `cle`, `search` and `inspect` were added.

## `spec --json`

Object: `repository`, `branch`, `commit`, `commit_date`, `retrieval_date`,
`spec_version`, `openapi`.

## `parse-tei <tei> --json`

Object: `domain`, `type`, `identifier`, `well_known`, `publication_issues`
(array of strings).

## `conformance [--json]`

The flag is accepted for uniformity; the report is always JSON. Object:
`spec_commit`, `spec_version`, `is_certification` (always `false`), `cases`
(array of `{id, passed, detail}`). It is an offline smoke report, not a
certification.

## `discover <tei> --json` and `discover --purl <purl> --base <url> --json`

Array of `{product_release, root_url, api_version}`: for each product release,
the server and API version to call next. For `--purl` that is chosen from the
`servers` of the `/discovery` answer on `--base`, as for a TEI. The PURL is
checked locally first (a malformed one is exit status 2 and nothing is sent)
and is sent exactly as given, not in canonical form.

## `cle --base <url> (--product | --product-release | --component | --component-release) <uuid> [--version <v>] --json`

```json
{
  "schema_version": 1,
  "subject": {"type": "product_release", "uuid": "<uuid as given>"},
  "implemented": true,
  "message": null,
  "cle": {"events": [ ... ], "definitions": { ... }},
  "version": "1.4.0",
  "status": {
    "released": "2024-01-01T00:00:00Z",
    "end_of_development": null,
    "end_of_support": null,
    "end_of_life": "2026-01-01T00:00:00Z",
    "end_of_distribution": null,
    "end_of_marketing": null,
    "superseded_by": null,
    "withdrawn_events": [4],
    "unevaluated": [{"event_id": 2, "reason": "unsupported_scheme", "detail": "pypi"}]
  }
}
```

- `subject.type` is `product`, `product_release`, `component` or
  `component_release`.
- `cle` is the CLE document re-serialised from the model (camelCase, as in the
  TEA OpenAPI). When the server answers `404 NOT_IMPLEMENTED`, `implemented`
  is `false`, `cle` is `null`, `message` is the server's message (or `null`)
  and the exit status is 0. Any other `404` is exit status 5.
- `version` and `status` are present only with `--version`, which takes a
  SemVer 2.0 version (`1.4.0`, not `1.4` or `v1.4.0`; anything else is exit
  status 2 and nothing is sent). `status` is `null` when `implemented` is
  `false`.
- Dates are the `effective` time of the event that applies (the highest `id`
  of each type); `null` means no applying event *that could be evaluated*.
  Events whose `vers` range is of a type other than `semver` or `npm`, that
  are malformed, or whose type is unknown are listed in `unevaluated` with
  `reason` `unsupported_scheme` (`detail`: the `vers` type), `malformed`
  (`detail`: what was wrong) or `unknown_event_type` (`detail`: the type).
  Check `unevaluated` before reading a `null` as "does not apply".
- `withdrawn_events` lists the ids cancelled by `withdrawn` events.

Without `--json`, `cle` prints the CLE document as indented JSON, or with
`--version` one `name<TAB>value` line per field above (`-` for none), plus
`withdrawn_event<TAB>id` and `unevaluated<TAB>id<TAB>reason<TAB>detail` lines,
and a warning on stderr when an event could not be evaluated.

## `product-release ... --json`

The Product Release object as the server sent it, re-serialised from the model
(`uuid`, `product`, `version`, `createdDate`, ..., camelCase as in the TEA
OpenAPI).

## `collections ... --json`

A Collection object (`--latest` or `--version`) or a page
(`hasNext`, `nextPageToken`, `results`), camelCase as in the TEA OpenAPI.

## `search <what> --base <url> [--id-type T [--id-value V]] ... --json`

`<what>` is `products`, `product-releases`, `components` or
`component-releases`. Output: one page as the server sent it, re-serialised
from the model, camelCase as in the TEA OpenAPI: `hasNext`, `nextPageToken`,
`results` (Product, Product Release, Component or Component Release objects).
Pagination is the caller's: pass `nextPageToken` back as `--page-token`.

- `--id-type` is `CPE`, `TEI`, `PURL` or `COMPLIANCE_DOCUMENT` (lowercase and
  `compliance-document` are accepted too). It can be given alone; TEA uses that
  form for compliance documents. `--id-value` needs `--id-type`.
- A `PURL` or `TEI` value is checked locally first (exit status 2 and nothing
  sent if it is malformed) and is sent exactly as given.
- `--sort-field` is `name` for products and components, and `created-date`,
  `release-date` or `version` for releases; `--sort-order` is `asc` or `desc`.
  Anything else is exit status 2.

Without `--json`: a line `has_next <bool><TAB>next_page_token <token>`, then one
line per result: `uuid<TAB>name` for products and components,
`uuid<TAB>version<TAB>product<TAB>product_name` for product releases and
`uuid<TAB>version<TAB>component<TAB>component_name` for component releases
(`-` when the name is absent).

## `inspect <tei> [--product-release <uuid>] [--collection-version N] --json`

A read-only summary: the TEI is resolved as for `sync` (several product releases
need `--product-release`, exit status 2 otherwise), then the product release,
its product and the latest collection (or `--collection-version`) are fetched.
No artifact is downloaded and no checksum is checked; to fetch and verify the
latest collection's artifacts and record them, use `sync`.

```json
{
  "schema_version": 1,
  "tei": "<tei as given>",
  "server": {"root_url": "https://api.example.com/", "api_version": "1.0.0"},
  "product": {"uuid": "...", "name": "Acme App",
              "identifiers": [{"type": "PURL", "value": "pkg:npm/acme-app"}]},
  "product_release": {"uuid": "...", "version": "2.1.0",
                      "created_date": "2024-01-01T00:00:00Z",
                      "release_date": "2024-01-15T00:00:00Z",
                      "components": [{"uuid": "...", "release": "..." }]},
  "collection": {"uuid": "...", "version": 3,
                 "created_date": "2024-02-01T00:00:00Z",
                 "update_reason": "ARTIFACT_ADDED",
                 "artifacts": [{"uuid": "...", "version": 2, "name": "SBOM", "type": "BOM",
                                "formats": [{"media_type": "application/vnd.cyclonedx+json",
                                             "checksum_algorithms": ["SHA-256"],
                                             "has_url": true, "has_signature": true}]}]}
}
```

- `server.root_url` has userinfo, query and fragment removed, as in a lock file.
- Artifact and signature URLs are not printed, because they can carry signed
  queries; `has_url` and `has_signature` say whether the format declares one.
  `has_signature` means a signature URL is published, not that anything was
  verified.
- `release_date`, a component's `release` and an artifact's `name` are `null`
  when absent.

Without `--json`: one tab-separated line per object, each starting with its
kind (`server`, `product`, `identifier`, `product_release`, `component`,
`collection`, `artifact`, `format`).

## `download ... --output FILE --json`

`--json` requires `--output`, so stdout carries only the JSON object. Without
`--output`, stdout is the artifact bytes (held in memory and printed only
after the checksum check) and `--json` is rejected with exit status 2.

```json
{
  "schema_version": 1,
  "artifact": "<artifact uuid as given>",
  "revision": 1,
  "media_type": "application/vnd.cyclonedx+json",
  "output": "<path as given>",
  "bytes_written": 1234,
  "content_type": "application/json",
  "content_encoding": null,
  "etag": "\"v1\"",
  "strong_etag": true,
  "final_url": "https://host/path",
  "integrity": {
    "outcome": "Verified",
    "scope": "decoded artifact bytes; not publisher authenticity",
    "checks": [
      {"algorithm": "SHA-256", "expected": "...", "actual": "...",
       "supported": true, "matched": true, "weak": false}
    ]
  }
}
```

- `revision` is `null` when the latest-revision endpoint was used.
- `final_url` has userinfo, query and fragment removed: a final URL can carry a
  signed query.
- `integrity.outcome` is one of `Verified`, `NotDeclared`, `Failed`,
  `Unsupported`, `Incomplete`, `Weak`, or `Unknown` for a value added later.
  The file exists only for `Verified` and `NotDeclared`. `NotDeclared` means
  nothing was checked.
- `Verified` is integrity of the decoded bytes. It is not publisher
  authenticity and no signature is checked.
- `etag` is a validator, not a checksum.

With `--signature` (added in 0.1.7), the object also has a `signature`
member. Without the flag the member is absent and no signature request is
made.

```json
"signature": {
  "status": "present",
  "format": "openpgp_armored",
  "bytes": 228,
  "content_type": "application/pgp-signature",
  "final_url": "https://host/path",
  "verified": false
}
```

- `status` is `present` or `not_published` (the server answered
  `SIGNATURE_NOT_FOUND`). For `not_published`, the other members are `null`
  except `verified`. Any other failure on the signature request is an error
  with the usual exit status; the artifact was already written by then.
- `format` is what the bytes look like, from
  `libtea_core::detect_signature_format`: `openpgp_armored`, `openpgp_binary`,
  `cms`, `sigstore_bundle`, `empty`, `too_large` (over 1 MiB, not inspected) or
  `unknown`. The server's `Content-Type` is reported but does not affect it.
- `verified` is always `false`. The CLI checks no signature, key or signer; a
  recognized format says nothing about whether the signature is valid.
- The signature bytes are not saved.

## Errors (`--json` given)

One line on stderr, nothing on stdout:

```json
{"schema_version": 1,
 "error": {"kind": "integrity", "exit_code": 6, "message": "..."}}
```

`kind` is one of `error` (1), `usage` (2), `auth` (3), `weak_checksums` (4),
`not_found` (5), `integrity` (6), `changes_detected` (7). For code 7 the differences are on stdout and this object is on stderr. `message` is
server-influenced text with control characters escaped (`\uXXXX`) and may
change between releases; match on `kind` or `exit_code`. If the command line
itself cannot be parsed, clap prints its usage text to stderr as plain text
(exit 2) before `--json` is known.

## `sync ... --json`

Without `--check`, one object on stdout:

```json
{"schema_version": 1, "lock": "tea.lock", "written": true,
 "previous": "none", "tei": "tei://...", "product_release": "...",
 "collection_version": 2, "artifacts": 5}
```

`previous` is `none` (no lock existed), `unchanged` (nothing but `fetchedAt`
differed, so the file was not rewritten and `written` is `false`) or `changed`.
The lock itself is described in `docs/lock-file.md`.

## `sync --check --json` and `diff old new --json`

One object on stdout, whether or not there are differences:

```json
{"schema_version": 1, "has_changes": true,
 "header":   [{"field": "collectionVersion", "old": "1", "new": "2"}],
 "added":    [ <lock entry> ],
 "removed":  [ <lock entry> ],
 "modified": [{"uuid": "...", "media_type": "...",
               "fields": [{"field": "size", "old": "3", "new": "4"}]}]}
```

The envelope is snake_case like the other CLI objects; `added` and `removed`
hold entries exactly as they appear in a lock file (camelCase,
`docs/lock-file.md`). `field` names inside `header` and `fields` are the lock
file's names (`collectionVersion`, `revision`, `size`, `checksums`, ...); `old`
and `new` are strings. Exit status 7 when `has_changes` is true, 0 otherwise.

## `cache stat --json` and `cache clear --json`

```json
{"schema_version": 1, "dir": "/home/me/.cache/libtea", "exists": true,
 "entries": 12, "bytes": 482113, "foreign": 0,
 "oldest_age_secs": 86400, "newest_age_secs": 12}
```

`foreign` counts files in the entry directory that are not cache entries.
Ages are by file time and are `null` for an empty cache; `exists: false` means
the directory has not been created yet. `clear` prints
`{"schema_version": 1, "dir": "...", "removed": N}`. A cache directory that is a
symbolic link, or that others can access, is exit status 1 for both.
