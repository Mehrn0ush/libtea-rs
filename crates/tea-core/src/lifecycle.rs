//! Lifecycle status of one version, derived from CLE (ECMA-428) events.
//!
//! **Experimental:** may change in a 0.x minor release.
//!
//! [`Cle::status_for_semver`] applies the events of a [`Cle`] to one SemVer
//! version, as `docs/design/cle.md` describes:
//!
//! 1. A `withdrawn` event cancels the event its `eventId` names.
//! 2. An event with no `versions` (and, for `released`, no `version`) applies
//!    to the component as a whole. Otherwise it applies when one of its
//!    specifiers matches: `{ version }` by SemVer equality, `{ range }` by
//!    [`VersRange::contains_semver`].
//! 3. Of the applying events of one type, the highest `id` wins.
//!
//! An event that cannot be evaluated (a `vers` type that is not SemVer, a
//! malformed specifier, an unknown event type) is never treated as "does not
//! apply": it is listed in [`LifecycleStatus::unevaluated`]. So "no end of
//! life recorded" and "end of life could not be evaluated" stay distinct.
//!
//! Dates are reported, never compared with the clock: whether a date is in
//! the past is for the caller to decide.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::enums::CleEventType;
use crate::ids::TeaTimestamp;
use crate::model::{Cle, CleEvent, CleVersionSpecifier};
use crate::vers::{VersError, VersRange};

/// The lifecycle state of one version.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// Each date is the `effective` time of the winning event of that type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct LifecycleStatus {
    /// `released`.
    pub released: Option<TeaTimestamp>,
    /// `endOfDevelopment`.
    pub end_of_development: Option<TeaTimestamp>,
    /// `endOfSupport`.
    pub end_of_support: Option<TeaTimestamp>,
    /// `endOfLife`.
    pub end_of_life: Option<TeaTimestamp>,
    /// `endOfDistribution`.
    pub end_of_distribution: Option<TeaTimestamp>,
    /// `endOfMarketing`.
    pub end_of_marketing: Option<TeaTimestamp>,
    /// `supersededByVersion` of the winning `supersededBy` event.
    pub superseded_by: Option<String>,
    /// Ids of events cancelled by a `withdrawn` event, ascending.
    pub withdrawn_events: Vec<i64>,
    /// Events that could not be evaluated for this version.
    pub unevaluated: Vec<Unevaluated>,
}

/// Why an event could not be evaluated.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Unevaluated {
    /// A `range` whose `vers` type is not compared as SemVer here.
    #[non_exhaustive]
    UnsupportedScheme {
        /// The event.
        event_id: i64,
        /// The `vers` type, such as `pypi`.
        scheme: String,
    },
    /// A specifier that does not parse or validate, or an event missing a
    /// field its type needs.
    #[non_exhaustive]
    Malformed {
        /// The event.
        event_id: i64,
        /// What was wrong.
        detail: String,
    },
    /// An event type this SDK does not know.
    #[non_exhaustive]
    UnknownEventType {
        /// The event.
        event_id: i64,
        /// The type as received.
        event_type: String,
    },
}

/// Outcome of matching one event against the version.
enum Applies {
    Yes,
    No,
    Unknown(Vec<Unevaluated>),
}

impl Cle {
    /// The lifecycle status of `version`, comparing versions as SemVer 2.0.
    ///
    /// **Experimental:** may change in a 0.x minor release.
    ///
    /// ```
    /// use libtea_core::Cle;
    ///
    /// let cle: Cle = serde_json::from_str(r#"{"events": [
    ///   {"id": 3, "type": "endOfLife", "effective": "2026-01-01T00:00:00Z",
    ///    "published": "2025-01-01T00:00:00Z", "versions": [{"range": "vers:semver/<2.0.0"}]},
    ///   {"id": 2, "type": "endOfSupport", "effective": "2025-06-01T00:00:00Z",
    ///    "published": "2025-01-01T00:00:00Z", "versions": [{"range": "vers:pypi/<2.0"}]},
    ///   {"id": 1, "type": "released", "effective": "2024-01-01T00:00:00Z",
    ///    "published": "2024-01-01T00:00:00Z", "version": "1.4.0"}
    /// ]}"#).unwrap();
    /// let status = cle.status_for_semver(&semver::Version::new(1, 4, 0));
    /// assert_eq!(status.end_of_life.unwrap().as_str(), "2026-01-01T00:00:00Z");
    /// assert_eq!(status.released.unwrap().as_str(), "2024-01-01T00:00:00Z");
    /// // Not "no end of support": the range could not be evaluated as SemVer.
    /// assert!(status.end_of_support.is_none());
    /// assert_eq!(status.unevaluated.len(), 1);
    /// ```
    #[must_use]
    pub fn status_for_semver(&self, version: &semver::Version) -> LifecycleStatus {
        let mut status = LifecycleStatus::default();
        let cancelled: BTreeSet<i64> = self
            .events
            .iter()
            .filter(|event| event.event_type == CleEventType::Withdrawn)
            .filter_map(|event| event.event_id)
            .collect();
        for event in &self.events {
            if event.event_type == CleEventType::Withdrawn && event.event_id.is_none() {
                status.unevaluated.push(Unevaluated::Malformed {
                    event_id: event.id,
                    detail: "withdrawn event has no eventId".into(),
                });
            }
        }
        status.withdrawn_events = self
            .events
            .iter()
            .map(|event| event.id)
            .filter(|id| cancelled.contains(id))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        // Highest id first, so the first applying event of a type wins.
        let mut events: Vec<&CleEvent> = self
            .events
            .iter()
            .filter(|event| event.event_type != CleEventType::Withdrawn)
            .filter(|event| !cancelled.contains(&event.id))
            .collect();
        events.sort_by_key(|event| std::cmp::Reverse(event.id));

        for event in events {
            if let CleEventType::Other(name) = &event.event_type {
                status.unevaluated.push(Unevaluated::UnknownEventType {
                    event_id: event.id,
                    event_type: name.clone(),
                });
                continue;
            }
            match applies(event, version) {
                Applies::No => continue,
                Applies::Unknown(reasons) => {
                    status.unevaluated.extend(reasons);
                    continue;
                }
                Applies::Yes => {}
            }
            let date = || Some(event.effective.clone());
            match &event.event_type {
                CleEventType::Released if status.released.is_none() => status.released = date(),
                CleEventType::EndOfDevelopment if status.end_of_development.is_none() => {
                    status.end_of_development = date();
                }
                CleEventType::EndOfSupport if status.end_of_support.is_none() => {
                    status.end_of_support = date();
                }
                CleEventType::EndOfLife if status.end_of_life.is_none() => {
                    status.end_of_life = date();
                }
                CleEventType::EndOfDistribution if status.end_of_distribution.is_none() => {
                    status.end_of_distribution = date();
                }
                CleEventType::EndOfMarketing if status.end_of_marketing.is_none() => {
                    status.end_of_marketing = date();
                }
                CleEventType::SupersededBy if status.superseded_by.is_none() => {
                    match &event.superseded_by_version {
                        Some(successor) => status.superseded_by = Some(successor.clone()),
                        None => status.unevaluated.push(Unevaluated::Malformed {
                            event_id: event.id,
                            detail: "supersededBy event has no supersededByVersion".into(),
                        }),
                    }
                }
                // `componentRenamed` changes identifiers, not a version's
                // lifecycle; a later event of a type already set loses.
                _ => {}
            }
        }
        status
    }
}

fn applies(event: &CleEvent, version: &semver::Version) -> Applies {
    let specifiers = event.versions.as_deref().unwrap_or_default();
    if event.event_type == CleEventType::Released {
        if let Some(released) = &event.version {
            return match semver::Version::parse(released) {
                Ok(released) if same_version(&released, version) => Applies::Yes,
                Ok(_) => Applies::No,
                Err(_) => Applies::Unknown(vec![malformed(event.id, "version is not SemVer")]),
            };
        }
    }
    if specifiers.is_empty() {
        return Applies::Yes;
    }
    let mut reasons = Vec::new();
    for specifier in specifiers {
        match matches(specifier, version, event.id) {
            Ok(true) => return Applies::Yes,
            Ok(false) => {}
            Err(reason) => reasons.push(reason),
        }
    }
    if reasons.is_empty() {
        Applies::No
    } else {
        Applies::Unknown(reasons)
    }
}

fn matches(
    specifier: &CleVersionSpecifier,
    version: &semver::Version,
    event_id: i64,
) -> Result<bool, Unevaluated> {
    match (&specifier.version, &specifier.range) {
        (Some(exact), None) => semver::Version::parse(exact)
            .map(|exact| same_version(&exact, version))
            .map_err(|_| malformed(event_id, "version is not SemVer")),
        (None, Some(range)) => {
            let parsed =
                VersRange::parse(range).map_err(|err| malformed(event_id, &err.to_string()))?;
            parsed.contains_semver(version).map_err(|err| match err {
                VersError::UnsupportedType => Unevaluated::UnsupportedScheme {
                    event_id,
                    scheme: parsed.version_type().to_owned(),
                },
                other => malformed(event_id, &other.to_string()),
            })
        }
        _ => Err(malformed(
            event_id,
            "a version specifier needs exactly one of version and range",
        )),
    }
}

fn same_version(a: &semver::Version, b: &semver::Version) -> bool {
    (a.major, a.minor, a.patch, &a.pre).cmp(&(b.major, b.minor, b.patch, &b.pre)) == Ordering::Equal
}

fn malformed(event_id: i64, detail: &str) -> Unevaluated {
    Unevaluated::Malformed {
        event_id,
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: i64, kind: &str, extra: serde_json::Value) -> serde_json::Value {
        let mut value = serde_json::json!({
            "id": id,
            "type": kind,
            "effective": format!("2026-01-{:02}T00:00:00Z", id),
            "published": "2025-01-01T00:00:00Z",
        });
        value
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        value
    }

    fn cle(events: Vec<serde_json::Value>) -> Cle {
        serde_json::from_value(serde_json::json!({ "events": events })).unwrap()
    }

    fn v(text: &str) -> semver::Version {
        semver::Version::parse(text).unwrap()
    }

    fn date(status: &Option<TeaTimestamp>) -> Option<&str> {
        status.as_ref().map(TeaTimestamp::as_str)
    }

    #[test]
    fn spec_examples() {
        // The two `cle-event` examples of the pinned OpenAPI.
        let doc: Cle = serde_json::from_value(serde_json::json!({"events": [
            {"id": 3, "type": "endOfSupport", "effective": "2024-01-01T00:00:00Z",
             "versions": [{"version": "1.0.0"}], "supportId": "standard",
             "published": "2023-06-01T00:00:00Z"},
            {"id": 1, "type": "released", "effective": "2024-01-01T00:00:00Z",
             "version": "1.0.0", "license": "MIT", "published": "2023-06-01T00:00:00Z"}
        ]}))
        .unwrap();
        let status = doc.status_for_semver(&v("1.0.0"));
        assert_eq!(date(&status.released), Some("2024-01-01T00:00:00Z"));
        assert_eq!(date(&status.end_of_support), Some("2024-01-01T00:00:00Z"));
        assert!(status.unevaluated.is_empty());
        let other = doc.status_for_semver(&v("1.0.1"));
        assert_eq!(other, LifecycleStatus::default());
    }

    #[test]
    fn highest_id_wins_and_component_wide_events_apply() {
        let doc = cle(vec![
            event(
                5,
                "endOfLife",
                serde_json::json!({"versions": [{"range": "vers:semver/<3.0.0"}]}),
            ),
            event(2, "endOfLife", serde_json::json!({})),
            event(4, "endOfMarketing", serde_json::json!({})),
        ]);
        let status = doc.status_for_semver(&v("2.0.0"));
        assert_eq!(date(&status.end_of_life), Some("2026-01-05T00:00:00Z"));
        assert_eq!(date(&status.end_of_marketing), Some("2026-01-04T00:00:00Z"));
        // 3.0.0 is outside the range of event 5, so the component-wide event 2 applies.
        let newer = doc.status_for_semver(&v("3.0.0"));
        assert_eq!(date(&newer.end_of_life), Some("2026-01-02T00:00:00Z"));
    }

    #[test]
    fn a_withdrawn_event_never_affects_the_status() {
        let target = event(
            3,
            "endOfSupport",
            serde_json::json!({"versions": [{"version": "1.0.0"}]}),
        );
        let older = event(1, "endOfSupport", serde_json::json!({}));
        let without = cle(vec![older.clone()]).status_for_semver(&v("1.0.0"));
        let with = cle(vec![
            event(
                4,
                "withdrawn",
                serde_json::json!({"eventId": 3, "reason": "typo"}),
            ),
            target,
            older,
        ])
        .status_for_semver(&v("1.0.0"));
        assert_eq!(with.end_of_support, without.end_of_support);
        assert_eq!(date(&with.end_of_support), Some("2026-01-01T00:00:00Z"));
        assert_eq!(with.withdrawn_events, vec![3]);
    }

    #[test]
    fn unevaluated_is_not_the_same_as_absent() {
        let doc = cle(vec![
            event(
                6,
                "endOfLife",
                serde_json::json!({"versions": [{"range": "vers:pypi/<2.0"}]}),
            ),
            event(
                5,
                "endOfSupport",
                serde_json::json!({"versions": [{"range": "vers:semver/>=2.0.0|<1.0.0"}]}),
            ),
            event(
                4,
                "endOfDevelopment",
                serde_json::json!({"versions": [{"version": "1.0"}]}),
            ),
            event(
                3,
                "endOfDistribution",
                serde_json::json!({"versions": [{}]}),
            ),
            event(2, "supersededBy", serde_json::json!({})),
            event(1, "futureEvent", serde_json::json!({})),
        ]);
        let status = doc.status_for_semver(&v("1.0.0"));
        assert_eq!(status.end_of_life, None);
        assert_eq!(status.superseded_by, None);
        let ids: Vec<i64> = status
            .unevaluated
            .iter()
            .map(|u| match u {
                Unevaluated::UnsupportedScheme { event_id, .. }
                | Unevaluated::Malformed { event_id, .. }
                | Unevaluated::UnknownEventType { event_id, .. } => *event_id,
            })
            .collect();
        assert_eq!(ids, vec![6, 5, 4, 3, 2, 1]);
        assert!(matches!(
            &status.unevaluated[0],
            Unevaluated::UnsupportedScheme { scheme, .. } if scheme == "pypi"
        ));
    }

    #[test]
    fn one_matching_specifier_is_enough() {
        let doc = cle(vec![event(
            2,
            "endOfLife",
            serde_json::json!({"versions": [{"range": "vers:pypi/<2.0"}, {"version": "1.2.0"}]}),
        )]);
        let status = doc.status_for_semver(&v("1.2.0"));
        assert_eq!(date(&status.end_of_life), Some("2026-01-02T00:00:00Z"));
        assert!(status.unevaluated.is_empty());
    }

    #[test]
    fn superseded_by_and_prerelease_and_build() {
        let doc = cle(vec![
            event(
                3,
                "supersededBy",
                serde_json::json!({"versions": [{"range": "vers:npm/<2.0.0"}], "supersededByVersion": "2.0.0"}),
            ),
            event(
                1,
                "released",
                serde_json::json!({"version": "2.0.0-rc.1+build.5"}),
            ),
        ]);
        let rc = doc.status_for_semver(&v("2.0.0-rc.1"));
        assert_eq!(rc.superseded_by.as_deref(), Some("2.0.0"));
        assert_eq!(date(&rc.released), Some("2026-01-01T00:00:00Z"));
        let release = doc.status_for_semver(&v("2.0.0"));
        assert_eq!(release.superseded_by, None);
        assert_eq!(release.released, None);
    }
}
