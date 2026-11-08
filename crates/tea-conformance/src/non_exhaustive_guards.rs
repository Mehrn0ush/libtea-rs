//! Doctest-only guards that `#[non_exhaustive]` stays on the listed items.
//!
//! This module is compiled only for `cargo test --doc` (`#[cfg(doctest)]`).
//! Each item has two structs:
//!
//! - `Guard…`: a `compile_fail` doctest. It must fail **only** because the
//!   item is `#[non_exhaustive]`. Enum guards match every current variant with
//!   no wildcard (so only `non_exhaustive` can make the match refutable).
//!   Struct guards use struct-update syntax with a base value
//!   (`T { field: base.field.clone(), ..base }`), so a missing field cannot
//!   be the cause; the only possible error is E0639. Struct-like enum variants
//!   list all their fields.
//! - `Control…`: a doctest that must compile. It proves the type path, the
//!   variant names, and the field names used by the guard are valid.
//!
//! Stable rustdoc does not check the error code of `compile_fail,E…`; the
//! code in the label is documentation. `scripts/check-non-exhaustive-guards.sh`
//! is the check that matters: it strips every `#[non_exhaustive]` in a scratch
//! copy and requires every guard to fail there (the snippet then compiles).
//! It also catches a guard that has rotted into failing for another reason
//! (for example after a variant is added to an enum).
//!
//!

/// `Report` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_conformance::Report) {
///     let _ = libtea_conformance::Report { spec_commit: base.spec_commit.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardReport;

/// Control for `GuardReport`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_conformance::Report) {
///     let kept = base.spec_commit.clone();
///     let libtea_conformance::Report { spec_commit, .. } = base;
///     let _ = (kept, spec_commit);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlReport;

/// `CaseResult` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_conformance::CaseResult) {
///     let _ = libtea_conformance::CaseResult { id: base.id.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCaseResult;

/// Control for `GuardCaseResult`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_conformance::CaseResult) {
///     let kept = base.id.clone();
///     let libtea_conformance::CaseResult { id, .. } = base;
///     let _ = (kept, id);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCaseResult;

/// `LintWarning` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_conformance::LintWarning) {
///     let _ = libtea_conformance::LintWarning { path: base.path.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardLintWarning;

/// Control for `GuardLintWarning`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_conformance::LintWarning) {
///     let kept = base.path.clone();
///     let libtea_conformance::LintWarning { path, .. } = base;
///     let _ = (kept, path);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlLintWarning;
