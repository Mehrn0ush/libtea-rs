//! Constructors and setters for `#[non_exhaustive]` configuration types.
//!
//! Struct literals no longer compile outside the crates (see the
//! `compile_fail` doctests in `src/lib.rs`), so these are the supported forms.

use std::time::Duration;

use libtea_client::{Limits, PageFollowLimits, ResolvedRelease};
use libtea_core::enums::{IdentifierType, ProductSort, SortOrder};
use libtea_core::{ChecksumPolicy, ListParams, ProductReleaseId};

#[test]
fn limits_setters_change_only_the_named_field() {
    let base = Limits::default();
    let tuned = Limits::default()
        .with_max_discovery_bytes(1)
        .with_max_json_bytes(2)
        .with_max_artifact_bytes(3)
        .with_max_redirects(4)
        .with_request_timeout(Duration::from_secs(5))
        .with_connect_timeout(Duration::from_secs(6))
        .with_max_retries(7)
        .with_max_discovery_attempts(8)
        .with_initial_backoff(Duration::from_millis(9))
        .with_max_in_flight(10);
    assert_eq!(tuned.max_discovery_bytes, 1);
    assert_eq!(tuned.max_json_bytes, 2);
    assert_eq!(tuned.max_artifact_bytes, 3);
    assert_eq!(tuned.max_redirects, 4);
    assert_eq!(tuned.request_timeout, Duration::from_secs(5));
    assert_eq!(tuned.connect_timeout, Duration::from_secs(6));
    assert_eq!(tuned.max_retries, 7);
    assert_eq!(tuned.max_discovery_attempts, 8);
    assert_eq!(tuned.initial_backoff, Duration::from_millis(9));
    assert_eq!(tuned.max_in_flight, 10);
    let one = Limits::default().with_max_retries(0);
    assert_eq!(one.max_retries, 0);
    assert_eq!(one.request_timeout, base.request_timeout);
}

#[test]
fn page_follow_limits_constructors() {
    let limits = PageFollowLimits::new(3, 40);
    assert_eq!((limits.max_pages, limits.max_items), (3, 40));
    let changed = PageFollowLimits::default()
        .with_max_pages(5)
        .with_max_items(6);
    assert_eq!((changed.max_pages, changed.max_items), (5, 6));
}

#[test]
fn list_params_setters() {
    let query = ListParams::<ProductSort>::default()
        .with_page_size(10)
        .with_page_token("t")
        .with_sort_field(ProductSort::Name)
        .with_sort_order(SortOrder::Desc)
        .with_id_type(IdentifierType::Purl)
        .with_id_value("pkg:generic/x@1");
    assert_eq!(query.page_size, Some(10));
    assert_eq!(query.page_token.as_deref(), Some("t"));
    assert_eq!(query.sort_field, Some(ProductSort::Name));
    assert_eq!(query.sort_order, Some(SortOrder::Desc));
    assert_eq!(query.id_type, Some(IdentifierType::Purl));
    assert_eq!(query.id_value.as_deref(), Some("pkg:generic/x@1"));
    assert!(query.validate().is_ok());
}

#[test]
fn resolved_release_and_checksum_policy_constructors() {
    let id = ProductReleaseId::parse("2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d").unwrap();
    let release = ResolvedRelease::new(id, "https://tea.example", "1.0.0");
    assert_eq!(release.root_url, "https://tea.example");
    assert_eq!(release.api_version, "1.0.0");
    assert!(!ChecksumPolicy::new().allow_weak);
    assert!(ChecksumPolicy::default().with_allow_weak(true).allow_weak);
}
