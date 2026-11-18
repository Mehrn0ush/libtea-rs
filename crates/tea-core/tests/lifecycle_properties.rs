//! Properties of `Cle::status_for_semver` and `VersRange::contains_semver`.

use libtea_core::vers::VersRange;
use libtea_core::Cle;
use proptest::prelude::*;

fn version() -> impl Strategy<Value = semver::Version> {
    (
        0u64..4,
        0u64..4,
        0u64..4,
        prop::option::of(prop::sample::select(vec!["rc.1", "alpha"])),
    )
        .prop_map(|(major, minor, patch, pre)| {
            let mut v = semver::Version::new(major, minor, patch);
            if let Some(pre) = pre {
                v.pre = semver::Prerelease::new(pre).unwrap();
            }
            v
        })
}

/// A valid range: sorted distinct versions with alternating lower and upper
/// bounds, so every generated range passes validation.
fn range() -> impl Strategy<Value = String> {
    (
        prop::collection::btree_set((0u64..4, 0u64..4, 0u64..4), 1..6),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(points, start_upper, inclusive)| {
            let mut upper = start_upper;
            let parts: Vec<String> = points
                .into_iter()
                .map(|(a, b, c)| {
                    let op = match (upper, inclusive) {
                        (true, true) => "<=",
                        (true, false) => "<",
                        (false, true) => ">=",
                        (false, false) => ">",
                    };
                    upper = !upper;
                    format!("{op}{a}.{b}.{c}")
                })
                .collect();
            format!("vers:semver/{}", parts.join("|"))
        })
}

fn event(id: i64, kind: &str, range: Option<&str>) -> serde_json::Value {
    let mut value = serde_json::json!({
        "id": id, "type": kind,
        "effective": format!("2026-01-{:02}T00:00:00Z", id),
        "published": "2025-01-01T00:00:00Z",
    });
    if let Some(range) = range {
        value["versions"] = serde_json::json!([{ "range": range }]);
    }
    value
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// A range never contains a version on both sides of the same bound
    /// inconsistently: it agrees with a brute-force interval check.
    #[test]
    fn containment_matches_a_reference(text in range(), v in version()) {
        let parsed = VersRange::parse(&text).unwrap();
        let inside = parsed.contains_semver(&v).unwrap();
        // Reference: walk the bounds; `v` is inside when the last bound at or
        // below it is a lower bound that admits it, or there is none and the
        // first bound is an upper bound that admits it.
        let key = |x: &semver::Version| (x.major, x.minor, x.patch, x.pre.clone());
        let mut expected = None;
        let bounds = parsed.constraints();
        for (index, c) in bounds.iter().enumerate() {
            let b = semver::Version::parse(c.version()).unwrap();
            let op = c.comparator().as_str();
            let upper = op.starts_with('<');
            if key(&v) < key(&b) || (key(&v) == key(&b) && !op.ends_with('=')) {
                if key(&v) == key(&b) {
                    expected = Some(false);
                } else {
                    expected = Some(upper && (index == 0 || !bounds[index - 1].comparator().as_str().starts_with('<')));
                }
                break;
            }
            if key(&v) == key(&b) {
                expected = Some(true);
                break;
            }
        }
        let expected = expected.unwrap_or_else(|| {
            !bounds.last().unwrap().comparator().as_str().starts_with('<')
        });
        prop_assert_eq!(inside, expected, "{} {}", text, v);
    }

    /// An event withdrawn by another never changes the status.
    #[test]
    fn withdrawn_events_never_count(text in range(), v in version(), kind in prop::sample::select(vec!["endOfLife", "endOfSupport", "released"])) {
        let base = vec![event(1, kind, None)];
        let mut with = base.clone();
        with.push(event(5, kind, Some(&text)));
        with.push(serde_json::json!({"id": 6, "type": "withdrawn", "eventId": 5,
            "effective": "2026-02-01T00:00:00Z", "published": "2026-02-01T00:00:00Z"}));
        let before: Cle = serde_json::from_value(serde_json::json!({"events": base})).unwrap();
        let after: Cle = serde_json::from_value(serde_json::json!({"events": with})).unwrap();
        let a = before.status_for_semver(&v);
        let b = after.status_for_semver(&v);
        prop_assert_eq!(&a.released, &b.released);
        prop_assert_eq!(&a.end_of_life, &b.end_of_life);
        prop_assert_eq!(&a.end_of_support, &b.end_of_support);
        prop_assert_eq!(&a.unevaluated, &b.unevaluated);
        prop_assert_eq!(b.withdrawn_events, vec![5]);
    }

    #[test]
    fn parsing_arbitrary_text_never_panics(text in "vers:[a-z]{0,4}/[<>=!*|%0-9a-z.+-]{0,30}", v in version()) {
        if let Ok(range) = VersRange::parse(&text) {
            let _ = range.contains_semver(&v);
        }
    }
}
