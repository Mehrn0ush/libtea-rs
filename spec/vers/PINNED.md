# Pinned `vers` test suite

| Field | Value |
| --- | --- |
| Upstream | https://github.com/package-url/vers-spec |
| Release | `v1.2.1` (2026-10-07) |
| Commit | `76c729180ce3ffb24b2e9f90e34585150fe0ffcf` |
| License | MIT (`spec/vers/LICENSE`, unmodified) |

Unmodified copies of the generic parse tests and the npm and PyPI
containment tests, used by `crates/tea-core/tests/vers_spec_suite.rs` to check
`libtea_core::vers`. The PyPI file was added for `VersRange::contains_pypi`
(#61), from the same commit. The other upstream test files cover version
comparison for schemes this SDK does not evaluate (Maven, Alpine, ...), or
the normalized `validate` form, which this SDK does not produce, and are not
vendored. `v1.2.2` (2026-11-17) was a day old when this was pinned; `v1.2.1`
was taken instead.

The normative text (`docs/specification/` upstream: `how-to-parse.md` and
clause 5 of the standard) is not vendored; `docs/design/cle.md` records what
it says and where it disagrees with the registered npm type.

`crates/tea-core/tests/vers_spec_suite.rs` checks every digest below.

| Path | Upstream path | SHA-256 |
| --- | --- | --- |
| `spec/vers/LICENSE` | `LICENSE` | `4c7db628084f2fb78ca1cd4f63643659a822e86c3a4493de682899f57884161a` |
| `spec/vers/tests/vers_canonical_parse_test.json` | `tests/vers_canonical_parse_test.json` | `2f771d9774bad55858872b193d964e33dc8d5727867e772d589f97f2ae0b1a75` |
| `spec/vers/tests/npm_range_containment_test.json` | `tests/npm_range_containment_test.json` | `84d968947c075227b0a9f047b7cf7e34a1d5f2969708978c35bf9c9f705bc664` |
| `spec/vers/tests/pypi_range_containment_test.json` | `tests/pypi_range_containment_test.json` | `e82b05ec514874e361a26d60e1fbd26814225ae4b6933bb644ad90d73accf721` |
