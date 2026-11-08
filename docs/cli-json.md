# CLI `--json` output

Every command accepts `--json`. Output goes to **stdout**; on failure stdout is
empty and one error object goes to **stderr** (see the end). Field names below
are asserted by `crates/tea-cli/tests/json_and_exit_codes.rs`; renaming one
fails a test. Fields are snake_case. New fields may be added in a minor release;
consumers should ignore unknown fields.

Objects introduced in 0.1.x carry `schema_version` (currently `1`). The older
outputs (`spec`, `parse-tei`, `discover`, `product-release`, `collections`,
`conformance`) keep the shapes they had in 0.1.0 and have no version field.

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

## `discover <tei> --json`

Array of `{product_release, root_url, api_version}`.

## `product-release ... --json`

The Product Release object as the server sent it, re-serialised from the model
(`uuid`, `product`, `version`, `createdDate`, ..., camelCase as in the TEA
OpenAPI).

## `collections ... --json`

A Collection object (`--latest` or `--version`) or a page
(`hasNext`, `nextPageToken`, `results`), camelCase as in the TEA OpenAPI.

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
