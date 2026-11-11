# CLE evaluation (ECMA-428), `vers` semver only: design

**Status:** proposal for issue #11. Nothing here is implemented. Review before
any code is written.

## What exists

`libtea-core` models the TEA projection of CLE (`Cle`, `CleEvent`,
`CleVersionSpecifier`, `CleDefinitions`) and `libtea-client` fetches it
(`CleFetch`). Nothing interprets the events: a caller who wants "is version
2.3.1 still supported?" reads the list and applies ECMA-428 itself.

## Rules from the pinned specification (`openapi.yaml`, `cle` and `cle-event`)

- A version specifier is exactly one of `{ version }` or `{ range }`. `range`
  is a `vers` string (`vers:npm/>=1.0.0|<2.0.0`). `version` has no scheme; it
  "maps to `vers:<scheme>/<version>` when the ecosystem is known".
- `events` are ordered by `id` descending. A `withdrawn` event (with `eventId`)
  cancels the event it names, and is included if and only if that event is.
- Events that name no version, other than `withdrawn`, apply to the component
  as a whole.
- On a release endpoint the server may omit events scoped to other versions.
- `200` with an empty `events` array: no event recorded. `404
  NOT_IMPLEMENTED`: the server does not provide CLE for the object.

## Proposed API (libtea-core, additive)

```rust
/// The lifecycle state of one version, derived from CLE events.
#[non_exhaustive]
pub struct LifecycleStatus {
    pub released: Option<TeaTimestamp>,
    pub end_of_development: Option<TeaTimestamp>,
    pub end_of_support: Option<TeaTimestamp>,
    pub end_of_life: Option<TeaTimestamp>,
    pub end_of_distribution: Option<TeaTimestamp>,
    pub end_of_marketing: Option<TeaTimestamp>,
    pub superseded_by: Option<String>,
    pub withdrawn_events: Vec<i64>,   // ids cancelled by `withdrawn`
    pub unevaluated: Vec<Unevaluated>, // events this evaluator could not apply
}

#[non_exhaustive]
pub enum Unevaluated {
    /// `vers:<scheme>/...` with a scheme other than `semver` (or one this
    /// version of the SDK does not support).
    UnsupportedScheme { event_id: i64, scheme: String },
    /// `{ version }` without a scheme and no scheme given by the caller.
    SchemeUnknown { event_id: i64 },
    /// The range or version does not parse.
    Malformed { event_id: i64, detail: String },
}

impl Cle {
    /// Evaluate the events for `version` under `scheme` (only "semver" is
    /// supported in stage 1). Never guesses a scheme.
    pub fn status_for(&self, version: &str, scheme: VersScheme) -> LifecycleStatus;
}
```

Times are the event's `effective` time. The evaluator reports dates; it does
not compare them with the clock. A caller decides what "now" is, which keeps it
deterministic and testable.

## Evaluation rules (stage 1)

1. Drop every event whose `id` is named by a `withdrawn` event's `eventId`;
   record the ids in `withdrawn_events`.
2. For each remaining event: if it has no `versions` (and no `version` for
   `released`), it applies. Otherwise it applies if any specifier matches the
   version:
   - `{ version: v }`: equal under semver when `scheme` is `semver` (build
     metadata ignored, as in SemVer 2.0.0); `SchemeUnknown` is not raised,
     because the caller supplied the scheme.
   - `{ range: "vers:semver/..." }`: evaluate the `vers` constraints
     (`=`, `!=`, `<`, `<=`, `>`, `>=`, `*`, `|`-separated) with the `semver`
     crate's comparison, which `libtea-core` already depends on.
   - `{ range: "vers:<other>/..." }`: `UnsupportedScheme`, not applied, and not
     treated as "does not match".
3. When several events of one type apply, the one with the highest `id` wins
   (ECMA-428 ordering).

An event that could not be evaluated is never silently treated as not applying:
it is listed in `unevaluated`, so "no end-of-life found" and "end-of-life could
not be evaluated" stay distinct.

## Decisions for you

1. **Name and placement.** `Cle::status_for` in `libtea-core`, plus a CLI
   command `libtea lifecycle <tei|--base --uuid> --version <v>` later. Or keep
   the CLI out of the first PR. Recommendation: library first.
2. **`vers` parser scope.** Only what `semver` needs, or a general `vers`
   parser that rejects other schemes after parsing. Recommendation: general
   parser for the syntax (it is small), semver comparison only.
3. **Version bump.** Additive, 0.1.x patch release.

## Tests and fuzzing

- The spec's examples (`cle-event` and `cle`) as table tests, plus hand-written
  cases: withdrawn events, a range with `|`, a `!=` exclusion, build metadata,
  pre-releases (`1.0.0-rc.1 < 1.0.0`), an unsupported scheme, a malformed range.
- Property test: an event withdrawn by a later event never affects the status.
- New fuzz target `vers_range`: arbitrary strings into the `vers` parser and the
  evaluator; no panic, and a parsed range re-parses to the same constraints.

The ECMA-428 text is not vendored here. The rules above are from the pinned
OpenAPI descriptions and the `vers` specification as I know it; checking them
against ECMA-428 is the first task of the implementation PR.
