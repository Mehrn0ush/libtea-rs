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
//! Items not listed here (the wire enums other than `CollectionOwner`) are covered by the same macro; `CollectionOwner` stands in for them.

/// `IntegrityOutcome` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::IntegrityOutcome) -> u8 {
///     match x {
///         libtea_core::IntegrityOutcome::Verified => 0,
///         libtea_core::IntegrityOutcome::Failed => 0,
///         libtea_core::IntegrityOutcome::Unsupported => 0,
///         libtea_core::IntegrityOutcome::Incomplete => 0,
///         libtea_core::IntegrityOutcome::NotDeclared => 0,
///         libtea_core::IntegrityOutcome::Weak => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardIntegrityOutcome;

/// Control for `GuardIntegrityOutcome`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::IntegrityOutcome) -> u8 {
///     match x {
///         libtea_core::IntegrityOutcome::Verified => 0,
///         libtea_core::IntegrityOutcome::Failed => 0,
///         libtea_core::IntegrityOutcome::Unsupported => 0,
///         libtea_core::IntegrityOutcome::Incomplete => 0,
///         libtea_core::IntegrityOutcome::NotDeclared => 0,
///         libtea_core::IntegrityOutcome::Weak => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlIntegrityOutcome;

/// `IpClass` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::IpClass) -> u8 {
///     match x {
///         libtea_core::IpClass::Public => 0,
///         libtea_core::IpClass::Loopback => 0,
///         libtea_core::IpClass::Private => 0,
///         libtea_core::IpClass::LinkLocal => 0,
///         libtea_core::IpClass::Documentation => 0,
///         libtea_core::IpClass::Multicast => 0,
///         libtea_core::IpClass::Reserved => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardIpClass;

/// Control for `GuardIpClass`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::IpClass) -> u8 {
///     match x {
///         libtea_core::IpClass::Public => 0,
///         libtea_core::IpClass::Loopback => 0,
///         libtea_core::IpClass::Private => 0,
///         libtea_core::IpClass::LinkLocal => 0,
///         libtea_core::IpClass::Documentation => 0,
///         libtea_core::IpClass::Multicast => 0,
///         libtea_core::IpClass::Reserved => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlIpClass;

/// `EnumKnowledge` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::enums::EnumKnowledge) -> u8 {
///     match x {
///         libtea_core::enums::EnumKnowledge::Known => 0,
///         libtea_core::enums::EnumKnowledge::Unknown => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardEnumKnowledge;

/// Control for `GuardEnumKnowledge`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::enums::EnumKnowledge) -> u8 {
///     match x {
///         libtea_core::enums::EnumKnowledge::Known => 0,
///         libtea_core::enums::EnumKnowledge::Unknown => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlEnumKnowledge;

/// `SortOrder` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::SortOrder) -> u8 {
///     match x {
///         libtea_core::SortOrder::Asc => 0,
///         libtea_core::SortOrder::Desc => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardSortOrder;

/// Control for `GuardSortOrder`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::SortOrder) -> u8 {
///     match x {
///         libtea_core::SortOrder::Asc => 0,
///         libtea_core::SortOrder::Desc => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlSortOrder;

/// `ProductSort` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::enums::ProductSort) -> u8 {
///     match x {
///         libtea_core::enums::ProductSort::Name => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardProductSort;

/// Control for `GuardProductSort`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::enums::ProductSort) -> u8 {
///     match x {
///         libtea_core::enums::ProductSort::Name => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlProductSort;

/// `ComponentSort` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::enums::ComponentSort) -> u8 {
///     match x {
///         libtea_core::enums::ComponentSort::Name => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardComponentSort;

/// Control for `GuardComponentSort`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::enums::ComponentSort) -> u8 {
///     match x {
///         libtea_core::enums::ComponentSort::Name => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlComponentSort;

/// `ReleaseSort` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::enums::ReleaseSort) -> u8 {
///     match x {
///         libtea_core::enums::ReleaseSort::CreatedDate => 0,
///         libtea_core::enums::ReleaseSort::ReleaseDate => 0,
///         libtea_core::enums::ReleaseSort::Version => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardReleaseSort;

/// Control for `GuardReleaseSort`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::enums::ReleaseSort) -> u8 {
///     match x {
///         libtea_core::enums::ReleaseSort::CreatedDate => 0,
///         libtea_core::enums::ReleaseSort::ReleaseDate => 0,
///         libtea_core::enums::ReleaseSort::Version => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlReleaseSort;

/// `CollectionSort` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::enums::CollectionSort) -> u8 {
///     match x {
///         libtea_core::enums::CollectionSort::Version => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCollectionSort;

/// Control for `GuardCollectionSort`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::enums::CollectionSort) -> u8 {
///     match x {
///         libtea_core::enums::CollectionSort::Version => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCollectionSort;

/// `CollectionOwner` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::enums::CollectionOwner) -> u8 {
///     match x {
///         libtea_core::enums::CollectionOwner::ComponentRelease => 0,
///         libtea_core::enums::CollectionOwner::ProductRelease => 0,
///         libtea_core::enums::CollectionOwner::Other(_) => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCollectionOwner;

/// Control for `GuardCollectionOwner`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::enums::CollectionOwner) -> u8 {
///     match x {
///         libtea_core::enums::CollectionOwner::ComponentRelease => 0,
///         libtea_core::enums::CollectionOwner::ProductRelease => 0,
///         libtea_core::enums::CollectionOwner::Other(_) => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCollectionOwner;

/// `TeiError` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::TeiError) -> u8 {
///     match x {
///         libtea_core::TeiError::Empty => 0,
///         libtea_core::TeiError::LegacyUrn => 0,
///         libtea_core::TeiError::Scheme => 0,
///         libtea_core::TeiError::Structure => 0,
///         libtea_core::TeiError::Domain => 0,
///         libtea_core::TeiError::Type => 0,
///         libtea_core::TeiError::Identifier => 0,
///         libtea_core::TeiError::PercentEncoding => 0,
///         libtea_core::TeiError::Registry { .. } => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardTeiError;

/// Control for `GuardTeiError`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::TeiError) -> u8 {
///     match x {
///         libtea_core::TeiError::Empty => 0,
///         libtea_core::TeiError::LegacyUrn => 0,
///         libtea_core::TeiError::Scheme => 0,
///         libtea_core::TeiError::Structure => 0,
///         libtea_core::TeiError::Domain => 0,
///         libtea_core::TeiError::Type => 0,
///         libtea_core::TeiError::Identifier => 0,
///         libtea_core::TeiError::PercentEncoding => 0,
///         libtea_core::TeiError::Registry { .. } => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlTeiError;

/// `ModelError` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::ModelError) -> u8 {
///     match x {
///         libtea_core::ModelError::Json(_) => 0,
///         libtea_core::ModelError::Invalid(_) => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardModelError;

/// Control for `GuardModelError`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::ModelError) -> u8 {
///     match x {
///         libtea_core::ModelError::Json(_) => 0,
///         libtea_core::ModelError::Invalid(_) => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlModelError;

/// `PolicyError` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::PolicyError) -> u8 {
///     match x {
///         libtea_core::PolicyError::Scheme(_) => 0,
///         libtea_core::PolicyError::Userinfo => 0,
///         libtea_core::PolicyError::QueryOrFragment => 0,
///         libtea_core::PolicyError::Host => 0,
///         libtea_core::PolicyError::TrailingSlash => 0,
///         libtea_core::PolicyError::Destination(_) => 0,
///         libtea_core::PolicyError::Hostname(_) => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardPolicyError;

/// Control for `GuardPolicyError`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::PolicyError) -> u8 {
///     match x {
///         libtea_core::PolicyError::Scheme(_) => 0,
///         libtea_core::PolicyError::Userinfo => 0,
///         libtea_core::PolicyError::QueryOrFragment => 0,
///         libtea_core::PolicyError::Host => 0,
///         libtea_core::PolicyError::TrailingSlash => 0,
///         libtea_core::PolicyError::Destination(_) => 0,
///         libtea_core::PolicyError::Hostname(_) => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlPolicyError;

/// `Identifier` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::Identifier) {
///     let _ = libtea_core::Identifier { id_type: base.id_type.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardIdentifier;

/// Control for `GuardIdentifier`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::Identifier) {
///     let kept = base.id_type.clone();
///     let libtea_core::Identifier { id_type, .. } = base;
///     let _ = (kept, id_type);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlIdentifier;

/// `Product` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::Product) {
///     let _ = libtea_core::Product { uuid: base.uuid.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardProduct;

/// Control for `GuardProduct`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::Product) {
///     let kept = base.uuid.clone();
///     let libtea_core::Product { uuid, .. } = base;
///     let _ = (kept, uuid);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlProduct;

/// `ComponentRef` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ComponentRef) {
///     let _ = libtea_core::ComponentRef { uuid: base.uuid.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardComponentRef;

/// Control for `GuardComponentRef`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ComponentRef) {
///     let kept = base.uuid.clone();
///     let libtea_core::ComponentRef { uuid, .. } = base;
///     let _ = (kept, uuid);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlComponentRef;

/// `ProductRelease` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ProductRelease) {
///     let _ = libtea_core::ProductRelease { uuid: base.uuid.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardProductRelease;

/// Control for `GuardProductRelease`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ProductRelease) {
///     let kept = base.uuid.clone();
///     let libtea_core::ProductRelease { uuid, .. } = base;
///     let _ = (kept, uuid);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlProductRelease;

/// `ProductReleaseWithCollection` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ProductReleaseWithCollection) {
///     let _ = libtea_core::ProductReleaseWithCollection { release: base.release.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardProductReleaseWithCollection;

/// Control for `GuardProductReleaseWithCollection`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ProductReleaseWithCollection) {
///     let kept = base.release.clone();
///     let libtea_core::ProductReleaseWithCollection { release, .. } = base;
///     let _ = (kept, release);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlProductReleaseWithCollection;

/// `Component` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::Component) {
///     let _ = libtea_core::Component { uuid: base.uuid.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardComponent;

/// Control for `GuardComponent`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::Component) {
///     let kept = base.uuid.clone();
///     let libtea_core::Component { uuid, .. } = base;
///     let _ = (kept, uuid);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlComponent;

/// `ReleaseDistribution` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::model::ReleaseDistribution) {
///     let _ = libtea_core::model::ReleaseDistribution { distribution_id: base.distribution_id.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardReleaseDistribution;

/// Control for `GuardReleaseDistribution`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::model::ReleaseDistribution) {
///     let kept = base.distribution_id.clone();
///     let libtea_core::model::ReleaseDistribution { distribution_id, .. } = base;
///     let _ = (kept, distribution_id);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlReleaseDistribution;

/// `ComponentRelease` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ComponentRelease) {
///     let _ = libtea_core::ComponentRelease { uuid: base.uuid.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardComponentRelease;

/// Control for `GuardComponentRelease`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ComponentRelease) {
///     let kept = base.uuid.clone();
///     let libtea_core::ComponentRelease { uuid, .. } = base;
///     let _ = (kept, uuid);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlComponentRelease;

/// `ComponentReleaseWithCollection` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ComponentReleaseWithCollection) {
///     let _ = libtea_core::ComponentReleaseWithCollection { release: base.release.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardComponentReleaseWithCollection;

/// Control for `GuardComponentReleaseWithCollection`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ComponentReleaseWithCollection) {
///     let kept = base.release.clone();
///     let libtea_core::ComponentReleaseWithCollection { release, .. } = base;
///     let _ = (kept, release);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlComponentReleaseWithCollection;

/// `CollectionUpdateReason` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::model::CollectionUpdateReason) {
///     let _ = libtea_core::model::CollectionUpdateReason { reason_type: base.reason_type.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCollectionUpdateReason;

/// Control for `GuardCollectionUpdateReason`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::model::CollectionUpdateReason) {
///     let kept = base.reason_type.clone();
///     let libtea_core::model::CollectionUpdateReason { reason_type, .. } = base;
///     let _ = (kept, reason_type);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCollectionUpdateReason;

/// `ArtifactFormat` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ArtifactFormat) {
///     let _ = libtea_core::ArtifactFormat { media_type: base.media_type.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardArtifactFormat;

/// Control for `GuardArtifactFormat`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ArtifactFormat) {
///     let kept = base.media_type.clone();
///     let libtea_core::ArtifactFormat { media_type, .. } = base;
///     let _ = (kept, media_type);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlArtifactFormat;

/// `Artifact` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::Artifact) {
///     let _ = libtea_core::Artifact { uuid: base.uuid.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardArtifact;

/// Control for `GuardArtifact`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::Artifact) {
///     let kept = base.uuid.clone();
///     let libtea_core::Artifact { uuid, .. } = base;
///     let _ = (kept, uuid);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlArtifact;

/// `Collection` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::Collection) {
///     let _ = libtea_core::Collection { uuid: base.uuid.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCollection;

/// Control for `GuardCollection`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::Collection) {
///     let kept = base.uuid.clone();
///     let libtea_core::Collection { uuid, .. } = base;
///     let _ = (kept, uuid);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCollection;

/// `TeaErrorBody` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::TeaErrorBody) {
///     let _ = libtea_core::TeaErrorBody { error: base.error.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardTeaErrorBody;

/// Control for `GuardTeaErrorBody`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::TeaErrorBody) {
///     let kept = base.error.clone();
///     let libtea_core::TeaErrorBody { error, .. } = base;
///     let _ = (kept, error);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlTeaErrorBody;

/// `TokenErrorBody` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::model::TokenErrorBody) {
///     let _ = libtea_core::model::TokenErrorBody { error: base.error.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardTokenErrorBody;

/// Control for `GuardTokenErrorBody`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::model::TokenErrorBody) {
///     let kept = base.error.clone();
///     let libtea_core::model::TokenErrorBody { error, .. } = base;
///     let _ = (kept, error);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlTokenErrorBody;

/// `TokenResponse` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::TokenResponse) {
///     let _ = libtea_core::TokenResponse { access_token: base.access_token.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardTokenResponse;

/// Control for `GuardTokenResponse`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::TokenResponse) {
///     let kept = base.access_token.clone();
///     let libtea_core::TokenResponse { access_token, .. } = base;
///     let _ = (kept, access_token);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlTokenResponse;

/// `CleSupportDefinition` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::model::CleSupportDefinition) {
///     let _ = libtea_core::model::CleSupportDefinition { id: base.id.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCleSupportDefinition;

/// Control for `GuardCleSupportDefinition`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::model::CleSupportDefinition) {
///     let kept = base.id.clone();
///     let libtea_core::model::CleSupportDefinition { id, .. } = base;
///     let _ = (kept, id);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCleSupportDefinition;

/// `CleDefinitions` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::model::CleDefinitions) {
///     let _ = libtea_core::model::CleDefinitions { support: base.support.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCleDefinitions;

/// Control for `GuardCleDefinitions`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::model::CleDefinitions) {
///     let kept = base.support.clone();
///     let libtea_core::model::CleDefinitions { support, .. } = base;
///     let _ = (kept, support);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCleDefinitions;

/// `CleEvent` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::model::CleEvent) {
///     let _ = libtea_core::model::CleEvent { id: base.id.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCleEvent;

/// Control for `GuardCleEvent`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::model::CleEvent) {
///     let kept = base.id.clone();
///     let libtea_core::model::CleEvent { id, .. } = base;
///     let _ = (kept, id);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCleEvent;

/// `CleVersionSpecifier` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::model::CleVersionSpecifier) {
///     let _ = libtea_core::model::CleVersionSpecifier { version: base.version.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCleVersionSpecifier;

/// Control for `GuardCleVersionSpecifier`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::model::CleVersionSpecifier) {
///     let kept = base.version.clone();
///     let libtea_core::model::CleVersionSpecifier { version, .. } = base;
///     let _ = (kept, version);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCleVersionSpecifier;

/// `Cle` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::Cle) {
///     let _ = libtea_core::Cle { events: base.events.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCle;

/// Control for `GuardCle`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::Cle) {
///     let kept = base.events.clone();
///     let libtea_core::Cle { events, .. } = base;
///     let _ = (kept, events);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCle;

/// `Page` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::Page<u8>) {
///     let _ = libtea_core::Page { has_next: base.has_next.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardPage;

/// Control for `GuardPage`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::Page<u8>) {
///     let kept = base.has_next.clone();
///     let libtea_core::Page { has_next, .. } = base;
///     let _ = (kept, has_next);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlPage;

/// `WellKnownEndpoint` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::discovery::WellKnownEndpoint) {
///     let _ = libtea_core::discovery::WellKnownEndpoint { url: base.url.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardWellKnownEndpoint;

/// Control for `GuardWellKnownEndpoint`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::discovery::WellKnownEndpoint) {
///     let kept = base.url.clone();
///     let libtea_core::discovery::WellKnownEndpoint { url, .. } = base;
///     let _ = (kept, url);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlWellKnownEndpoint;

/// `DiscoveryInfo` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::discovery::DiscoveryInfo) {
///     let _ = libtea_core::discovery::DiscoveryInfo { product_release: base.product_release.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardDiscoveryInfo;

/// Control for `GuardDiscoveryInfo`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::discovery::DiscoveryInfo) {
///     let kept = base.product_release.clone();
///     let libtea_core::discovery::DiscoveryInfo { product_release, .. } = base;
///     let _ = (kept, product_release);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlDiscoveryInfo;

/// `DiscoveryServer` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::discovery::DiscoveryServer) {
///     let _ = libtea_core::discovery::DiscoveryServer { root_url: base.root_url.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardDiscoveryServer;

/// Control for `GuardDiscoveryServer`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::discovery::DiscoveryServer) {
///     let kept = base.root_url.clone();
///     let libtea_core::discovery::DiscoveryServer { root_url, .. } = base;
///     let _ = (kept, root_url);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlDiscoveryServer;

/// `SelectedEndpoint` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::SelectedEndpoint) {
///     let _ = libtea_core::SelectedEndpoint { root_url: base.root_url.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardSelectedEndpoint;

/// Control for `GuardSelectedEndpoint`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::SelectedEndpoint) {
///     let kept = base.root_url.clone();
///     let libtea_core::SelectedEndpoint { root_url, .. } = base;
///     let _ = (kept, root_url);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlSelectedEndpoint;

/// `VersionedEndpoint` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::version::VersionedEndpoint) {
///     let _ = libtea_core::version::VersionedEndpoint { root_url: base.root_url.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardVersionedEndpoint;

/// Control for `GuardVersionedEndpoint`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::version::VersionedEndpoint) {
///     let kept = base.root_url.clone();
///     let libtea_core::version::VersionedEndpoint { root_url, .. } = base;
///     let _ = (kept, root_url);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlVersionedEndpoint;

/// `ListParams` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ListParams<()>) {
///     let _ = libtea_core::ListParams { page_size: base.page_size.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardListParams;

/// Control for `GuardListParams`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ListParams<()>) {
///     let kept = base.page_size.clone();
///     let libtea_core::ListParams { page_size, .. } = base;
///     let _ = (kept, page_size);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlListParams;

/// `IntegrityReport` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::IntegrityReport) {
///     let _ = libtea_core::IntegrityReport { outcome: base.outcome.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardIntegrityReport;

/// Control for `GuardIntegrityReport`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::IntegrityReport) {
///     let kept = base.outcome.clone();
///     let libtea_core::IntegrityReport { outcome, .. } = base;
///     let _ = (kept, outcome);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlIntegrityReport;

/// `ChecksumCheck` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ChecksumCheck) {
///     let _ = libtea_core::ChecksumCheck { algorithm: base.algorithm.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardChecksumCheck;

/// Control for `GuardChecksumCheck`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ChecksumCheck) {
///     let kept = base.algorithm.clone();
///     let libtea_core::ChecksumCheck { algorithm, .. } = base;
///     let _ = (kept, algorithm);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlChecksumCheck;

/// `ChecksumPolicy` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ChecksumPolicy) {
///     let _ = libtea_core::ChecksumPolicy { allow_weak: base.allow_weak.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardChecksumPolicy;

/// Control for `GuardChecksumPolicy`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ChecksumPolicy) {
///     let kept = base.allow_weak.clone();
///     let libtea_core::ChecksumPolicy { allow_weak, .. } = base;
///     let _ = (kept, allow_weak);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlChecksumPolicy;

/// `libtea_core::TeiError::Registry` is a `#[non_exhaustive]` variant: a literal with every field must not compile.
///
/// ```compile_fail,E0639
/// fn f(tei_type: String, reason: String) {
///     let _ = libtea_core::TeiError::Registry { tei_type, reason };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardTeiErrorRegistry;

/// Control for `GuardTeiErrorRegistry`: a pattern with `..` compiles.
///
/// ```
/// fn f(x: libtea_core::TeiError) {
///     if let libtea_core::TeiError::Registry { tei_type, reason, .. } = x {
///         let _ = (tei_type, reason,);
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlTeiErrorRegistry;

/// Control: `url`, `semver`, and `uuid` are re-exported and name the types in the public API.
///
/// ```
/// use libtea_core::semver::Version;
/// use libtea_core::url::Url;
/// use libtea_core::uuid::Uuid;
/// let _ = (Url::parse("https://tea.example/").unwrap(), Version::new(1, 0, 0), Uuid::nil());
/// ```
#[allow(dead_code)]
pub(crate) struct ControlReexports;

/// `LockedIntegrity` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::LockedIntegrity) -> u8 {
///     match x {
///         libtea_core::LockedIntegrity::Verified => 0,
///         libtea_core::LockedIntegrity::VerifiedWeakOnly => 0,
///         libtea_core::LockedIntegrity::NotDeclared => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardLockedIntegrity;

/// Control for `GuardLockedIntegrity`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::LockedIntegrity) -> u8 {
///     match x {
///         libtea_core::LockedIntegrity::Verified => 0,
///         libtea_core::LockedIntegrity::VerifiedWeakOnly => 0,
///         libtea_core::LockedIntegrity::NotDeclared => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlLockedIntegrity;

/// `LockFile` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::LockFile) {
///     let _ = libtea_core::LockFile { tei: base.tei.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardLockFile;

/// Control for `GuardLockFile`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::LockFile) {
///     let kept = base.tei.clone();
///     let libtea_core::LockFile { tei, .. } = base;
///     let _ = (kept, tei);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlLockFile;

/// `LockedArtifact` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::LockedArtifact) {
///     let _ = libtea_core::LockedArtifact { uuid: base.uuid.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardLockedArtifact;

/// Control for `GuardLockedArtifact`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::LockedArtifact) {
///     let kept = base.uuid.clone();
///     let libtea_core::LockedArtifact { uuid, .. } = base;
///     let _ = (kept, uuid);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlLockedArtifact;

/// `LockedChecksum` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::LockedChecksum) {
///     let _ = libtea_core::LockedChecksum { algorithm: base.algorithm.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardLockedChecksum;

/// Control for `GuardLockedChecksum`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::LockedChecksum) {
///     let kept = base.algorithm.clone();
///     let libtea_core::LockedChecksum { algorithm, .. } = base;
///     let _ = (kept, algorithm);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlLockedChecksum;

/// `FieldChange` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::FieldChange) {
///     let _ = libtea_core::FieldChange { field: base.field.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardFieldChange;

/// Control for `GuardFieldChange`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::FieldChange) {
///     let kept = base.field.clone();
///     let libtea_core::FieldChange { field, .. } = base;
///     let _ = (kept, field);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlFieldChange;

/// `ChangedArtifact` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::ChangedArtifact) {
///     let _ = libtea_core::ChangedArtifact { uuid: base.uuid.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardChangedArtifact;

/// Control for `GuardChangedArtifact`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::ChangedArtifact) {
///     let kept = base.uuid.clone();
///     let libtea_core::ChangedArtifact { uuid, .. } = base;
///     let _ = (kept, uuid);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlChangedArtifact;

/// `LockDiff` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::LockDiff) {
///     let _ = libtea_core::LockDiff { header: base.header.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardLockDiff;

/// Control for `GuardLockDiff`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::LockDiff) {
///     let kept = base.header.clone();
///     let libtea_core::LockDiff { header, .. } = base;
///     let _ = (kept, header);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlLockDiff;

/// `CacheControl` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::http_cache::CacheControl) {
///     let _ = libtea_core::http_cache::CacheControl { no_store: base.no_store, ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCacheControl;

/// Control for `GuardCacheControl`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::http_cache::CacheControl) {
///     let kept = base.no_store.clone();
///     let libtea_core::http_cache::CacheControl { no_store, .. } = base;
///     let _ = (kept, no_store);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCacheControl;

/// `CacheEntryMeta` is `#[non_exhaustive]`: struct-update syntax must not compile.
///
/// ```compile_fail,E0639
/// fn f(base: libtea_core::http_cache::CacheEntryMeta) {
///     let _ = libtea_core::http_cache::CacheEntryMeta { key: base.key.clone(), ..base };
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardCacheEntryMeta;

/// Control for `GuardCacheEntryMeta`: the field read and a `..` pattern compile.
///
/// ```
/// fn f(base: libtea_core::http_cache::CacheEntryMeta) {
///     let kept = base.key.clone();
///     let libtea_core::http_cache::CacheEntryMeta { key, .. } = base;
///     let _ = (kept, key);
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlCacheEntryMeta;

/// `NotStored` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::http_cache::NotStored) -> u8 {
///     match x {
///         libtea_core::http_cache::NotStored::Status => 0,
///         libtea_core::http_cache::NotStored::NoStore => 0,
///         libtea_core::http_cache::NotStored::Private => 0,
///         libtea_core::http_cache::NotStored::Authenticated => 0,
///         libtea_core::http_cache::NotStored::VaryStar => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardNotStored;

/// Control for `GuardNotStored`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::http_cache::NotStored) -> u8 {
///     match x {
///         libtea_core::http_cache::NotStored::Status => 0,
///         libtea_core::http_cache::NotStored::NoStore => 0,
///         libtea_core::http_cache::NotStored::Private => 0,
///         libtea_core::http_cache::NotStored::Authenticated => 0,
///         libtea_core::http_cache::NotStored::VaryStar => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlNotStored;

/// `EntryError` is `#[non_exhaustive]`: a match on every variant without a wildcard must not compile.
///
/// ```compile_fail,E0004
/// fn f(x: libtea_core::http_cache::EntryError) -> u8 {
///     match x {
///         libtea_core::http_cache::EntryError::Format => 0,
///         libtea_core::http_cache::EntryError::Meta => 0,
///         libtea_core::http_cache::EntryError::KeyMismatch => 0,
///         libtea_core::http_cache::EntryError::Body => 0,
///         libtea_core::http_cache::EntryError::TooLarge => 0,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct GuardEntryError;

/// Control for `GuardEntryError`: the same arms plus a wildcard compile.
///
/// ```
/// fn f(x: libtea_core::http_cache::EntryError) -> u8 {
///     match x {
///         libtea_core::http_cache::EntryError::Format => 0,
///         libtea_core::http_cache::EntryError::Meta => 0,
///         libtea_core::http_cache::EntryError::KeyMismatch => 0,
///         libtea_core::http_cache::EntryError::Body => 0,
///         libtea_core::http_cache::EntryError::TooLarge => 0,
///         _ => 1,
///     }
/// }
/// ```
#[allow(dead_code)]
pub(crate) struct ControlEntryError;
