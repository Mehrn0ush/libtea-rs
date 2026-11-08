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

/// `ClientError` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_client::ClientError) -> u8 {
///     match x {
///         libtea_client::ClientError::Policy(_) => 0,
///         libtea_client::ClientError::Model(_) => 0,
///         libtea_client::ClientError::Json(_) => 0,
///         libtea_client::ClientError::Transport(_) => 0,
///         libtea_client::ClientError::TooLarge => 0,
///         libtea_client::ClientError::TooManyRedirects => 0,
///         libtea_client::ClientError::BadRedirect => 0,
///         libtea_client::ClientError::Timeout => 0,
///         libtea_client::ClientError::AuthenticationRequired => 0,
///         libtea_client::ClientError::Forbidden => 0,
///         libtea_client::ClientError::Api { .. } => 0,
///         libtea_client::ClientError::Unresolved { .. } => 0,
///         libtea_client::ClientError::DiscoveryFailed { .. } => 0,
///         libtea_client::ClientError::Integrity { .. } => 0,
///         libtea_client::ClientError::Token { .. } => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardClientError;

/// Control for `GuardClientError`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_client::ClientError) -> u8 {
///     match x {
///         libtea_client::ClientError::Policy(_) => 0,
///         libtea_client::ClientError::Model(_) => 0,
///         libtea_client::ClientError::Json(_) => 0,
///         libtea_client::ClientError::Transport(_) => 0,
///         libtea_client::ClientError::TooLarge => 0,
///         libtea_client::ClientError::TooManyRedirects => 0,
///         libtea_client::ClientError::BadRedirect => 0,
///         libtea_client::ClientError::Timeout => 0,
///         libtea_client::ClientError::AuthenticationRequired => 0,
///         libtea_client::ClientError::Forbidden => 0,
///         libtea_client::ClientError::Api { .. } => 0,
///         libtea_client::ClientError::Unresolved { .. } => 0,
///         libtea_client::ClientError::DiscoveryFailed { .. } => 0,
///         libtea_client::ClientError::Integrity { .. } => 0,
///         libtea_client::ClientError::Token { .. } => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlClientError;

/// `PageFollowStop` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_client::PageFollowStop) -> u8 {
///     match x {
///         libtea_client::PageFollowStop::Exhausted => 0,
///         libtea_client::PageFollowStop::MaxPages => 0,
///         libtea_client::PageFollowStop::MaxItems => 0,
///         libtea_client::PageFollowStop::RepeatedToken => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardPageFollowStop;

/// Control for `GuardPageFollowStop`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_client::PageFollowStop) -> u8 {
///     match x {
///         libtea_client::PageFollowStop::Exhausted => 0,
///         libtea_client::PageFollowStop::MaxPages => 0,
///         libtea_client::PageFollowStop::MaxItems => 0,
///         libtea_client::PageFollowStop::RepeatedToken => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlPageFollowStop;

/// `SignatureVerifyOutcome` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_client::SignatureVerifyOutcome) -> u8 {
///     match x {
///         libtea_client::SignatureVerifyOutcome::Verified => 0,
///         libtea_client::SignatureVerifyOutcome::Invalid => 0,
///         libtea_client::SignatureVerifyOutcome::Unsupported => 0,
///         libtea_client::SignatureVerifyOutcome::NoTrustAnchor => 0,
///         libtea_client::SignatureVerifyOutcome::MissingSignature => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardSignatureVerifyOutcome;

/// Control for `GuardSignatureVerifyOutcome`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_client::SignatureVerifyOutcome) -> u8 {
///     match x {
///         libtea_client::SignatureVerifyOutcome::Verified => 0,
///         libtea_client::SignatureVerifyOutcome::Invalid => 0,
///         libtea_client::SignatureVerifyOutcome::Unsupported => 0,
///         libtea_client::SignatureVerifyOutcome::NoTrustAnchor => 0,
///         libtea_client::SignatureVerifyOutcome::MissingSignature => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlSignatureVerifyOutcome;

/// `ArtifactFetch` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_client::ArtifactFetch) -> u8 {
///     match x {
///         libtea_client::ArtifactFetch::Body(_) => 0,
///         libtea_client::ArtifactFetch::NotModified { .. } => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardArtifactFetch;

/// Control for `GuardArtifactFetch`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_client::ArtifactFetch) -> u8 {
///     match x {
///         libtea_client::ArtifactFetch::Body(_) => 0,
///         libtea_client::ArtifactFetch::NotModified { .. } => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlArtifactFetch;

/// `CleFetch` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_client::CleFetch) -> u8 {
///     match x {
///         libtea_client::CleFetch::Events(_) => 0,
///         libtea_client::CleFetch::NotImplemented { .. } => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCleFetch;

/// Control for `GuardCleFetch`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_client::CleFetch) -> u8 {
///     match x {
///         libtea_client::CleFetch::Events(_) => 0,
///         libtea_client::CleFetch::NotImplemented { .. } => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCleFetch;

/// `SignatureFetch` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_client::SignatureFetch) -> u8 {
///     match x {
///         libtea_client::SignatureFetch::Present(_) => 0,
///         libtea_client::SignatureFetch::NotPublished => 0,
///         libtea_client::SignatureFetch::NotModified { .. } => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardSignatureFetch;

/// Control for `GuardSignatureFetch`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_client::SignatureFetch) -> u8 {
///     match x {
///         libtea_client::SignatureFetch::Present(_) => 0,
///         libtea_client::SignatureFetch::NotPublished => 0,
///         libtea_client::SignatureFetch::NotModified { .. } => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlSignatureFetch;

/// `Limits` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_client::Limits) {
///     let _ = libtea_client::Limits { max_discovery_bytes: base.max_discovery_bytes.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardLimits;

/// Control for `GuardLimits`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_client::Limits) {
///     let kept = base.max_discovery_bytes.clone();
///     let libtea_client::Limits { max_discovery_bytes, .. } = base;
///     let _ = (kept, max_discovery_bytes);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlLimits;

/// `PageFollowLimits` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_client::PageFollowLimits) {
///     let _ = libtea_client::PageFollowLimits { max_pages: base.max_pages.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardPageFollowLimits;

/// Control for `GuardPageFollowLimits`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_client::PageFollowLimits) {
///     let kept = base.max_pages.clone();
///     let libtea_client::PageFollowLimits { max_pages, .. } = base;
///     let _ = (kept, max_pages);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlPageFollowLimits;

/// `FollowedPage` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_client::FollowedPage<u8>) {
///     let _ = libtea_client::FollowedPage { stop: base.stop.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardFollowedPage;

/// Control for `GuardFollowedPage`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_client::FollowedPage<u8>) {
///     let kept = base.stop.clone();
///     let libtea_client::FollowedPage { stop, .. } = base;
///     let _ = (kept, stop);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlFollowedPage;

/// `ResolvedRelease` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_client::ResolvedRelease) {
///     let _ = libtea_client::ResolvedRelease { root_url: base.root_url.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardResolvedRelease;

/// Control for `GuardResolvedRelease`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_client::ResolvedRelease) {
///     let kept = base.root_url.clone();
///     let libtea_client::ResolvedRelease { root_url, .. } = base;
///     let _ = (kept, root_url);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlResolvedRelease;

/// `DownloadedArtifact` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_client::DownloadedArtifact) {
///     let _ = libtea_client::DownloadedArtifact { bytes: base.bytes.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardDownloadedArtifact;

/// Control for `GuardDownloadedArtifact`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_client::DownloadedArtifact) {
///     let kept = base.bytes.clone();
///     let libtea_client::DownloadedArtifact { bytes, .. } = base;
///     let _ = (kept, bytes);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlDownloadedArtifact;

/// `libtea_client::ClientError::Api` is a `#[non_exhaustive]` variant: a literal with every field must not compile.
///
/// ```compile_fail,E0639
/// fn f(status: u16, tea_error: Option<String>, message: Option<String>) {
///     let _ = libtea_client::ClientError::Api { status, tea_error, message };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardClientErrorApi;

/// Control for `GuardClientErrorApi`: a pattern with `..` compiles.
///
/// ```
/// fn f(x: libtea_client::ClientError) {
///     if let libtea_client::ClientError::Api { status, tea_error, message, .. } = x {
///         let _ = (status, tea_error, message,);
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlClientErrorApi;

/// `libtea_client::ClientError::Unresolved` is a `#[non_exhaustive]` variant: a literal with every field must not compile.
///
/// ```compile_fail,E0639
/// fn f(error: String, message: Option<String>) {
///     let _ = libtea_client::ClientError::Unresolved { error, message };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardClientErrorUnresolved;

/// Control for `GuardClientErrorUnresolved`: a pattern with `..` compiles.
///
/// ```
/// fn f(x: libtea_client::ClientError) {
///     if let libtea_client::ClientError::Unresolved { error, message, .. } = x {
///         let _ = (error, message,);
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlClientErrorUnresolved;

/// `libtea_client::ClientError::DiscoveryFailed` is a `#[non_exhaustive]` variant: a literal with every field must not compile.
///
/// ```compile_fail,E0639
/// fn f(stage: &'static str, attempts: u32) {
///     let _ = libtea_client::ClientError::DiscoveryFailed { stage, attempts };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardClientErrorDiscoveryFailed;

/// Control for `GuardClientErrorDiscoveryFailed`: a pattern with `..` compiles.
///
/// ```
/// fn f(x: libtea_client::ClientError) {
///     if let libtea_client::ClientError::DiscoveryFailed { stage, attempts, .. } = x {
///         let _ = (stage, attempts,);
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlClientErrorDiscoveryFailed;

/// `libtea_client::ClientError::Integrity` is a `#[non_exhaustive]` variant: a literal with every field must not compile.
///
/// ```compile_fail,E0639
/// fn f(outcome: libtea_core::IntegrityOutcome, report: libtea_core::IntegrityReport) {
///     let _ = libtea_client::ClientError::Integrity { outcome, report };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardClientErrorIntegrity;

/// Control for `GuardClientErrorIntegrity`: a pattern with `..` compiles.
///
/// ```
/// fn f(x: libtea_client::ClientError) {
///     if let libtea_client::ClientError::Integrity { outcome, report, .. } = x {
///         let _ = (outcome, report,);
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlClientErrorIntegrity;

/// `libtea_client::ClientError::Token` is a `#[non_exhaustive]` variant: a literal with every field must not compile.
///
/// ```compile_fail,E0639
/// fn f(error: String) {
///     let _ = libtea_client::ClientError::Token { error };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardClientErrorToken;

/// Control for `GuardClientErrorToken`: a pattern with `..` compiles.
///
/// ```
/// fn f(x: libtea_client::ClientError) {
///     if let libtea_client::ClientError::Token { error, .. } = x {
///         let _ = (error,);
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlClientErrorToken;

/// `libtea_client::ArtifactFetch::NotModified` is a `#[non_exhaustive]` variant: a literal with every field must not compile.
///
/// ```compile_fail,E0639
/// fn f(etag: Option<String>, cache_control: Option<String>, vary: Option<String>) {
///     let _ = libtea_client::ArtifactFetch::NotModified { etag, cache_control, vary };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardArtifactFetchNotModified;

/// Control for `GuardArtifactFetchNotModified`: a pattern with `..` compiles.
///
/// ```
/// fn f(x: libtea_client::ArtifactFetch) {
///     if let libtea_client::ArtifactFetch::NotModified { etag, cache_control, vary, .. } = x {
///         let _ = (etag, cache_control, vary,);
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlArtifactFetchNotModified;

/// `libtea_client::CleFetch::NotImplemented` is a `#[non_exhaustive]` variant: a literal with every field must not compile.
///
/// ```compile_fail,E0639
/// fn f(message: Option<String>) {
///     let _ = libtea_client::CleFetch::NotImplemented { message };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCleFetchNotImplemented;

/// Control for `GuardCleFetchNotImplemented`: a pattern with `..` compiles.
///
/// ```
/// fn f(x: libtea_client::CleFetch) {
///     if let libtea_client::CleFetch::NotImplemented { message, .. } = x {
///         let _ = (message,);
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCleFetchNotImplemented;

/// `libtea_client::SignatureFetch::NotModified` is a `#[non_exhaustive]` variant: a literal with every field must not compile.
///
/// ```compile_fail,E0639
/// fn f(etag: Option<String>) {
///     let _ = libtea_client::SignatureFetch::NotModified { etag };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardSignatureFetchNotModified;

/// Control for `GuardSignatureFetchNotModified`: a pattern with `..` compiles.
///
/// ```
/// fn f(x: libtea_client::SignatureFetch) {
///     if let libtea_client::SignatureFetch::NotModified { etag, .. } = x {
///         let _ = (etag,);
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlSignatureFetchNotModified;
