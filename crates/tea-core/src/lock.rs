//! Lock file: a record of what a TEI resolved to and what the artifacts were.
//!
//! *Experimental until 0.9.* The JSON layout is versioned by `schemaVersion`
//! (currently [`LOCK_SCHEMA_VERSION`]); the Rust types may change in a minor
//! release.
//!
//! The format is documented in `docs/lock-file.md` and described by
//! `docs/schemas/lock-v1.schema.json`. This module does no I/O and reads no
//! clock: callers supply every value, including `fetchedAt`.
//!
//! **What a lock proves.** It records the integrity outcome of checksums over
//! decoded artifact bytes at `fetchedAt`. That is not publisher authenticity,
//! and a lock file is not signed: whoever can edit it can change it.
//!
//! **No secrets.** [`LockFile::validate`] rejects a `server` URL that carries
//! userinfo, a query or a fragment. Nothing else in the format holds
//! credentials.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::checksum::Checksum;
use crate::error::ModelError;
use crate::ids::{ArtifactId, ProductReleaseId, TeaTimestamp};

/// The only lock layout this crate reads and writes.
pub const LOCK_SCHEMA_VERSION: u64 = 1;

/// How an artifact's declared checksums fared when the lock was written.
///
/// Only outcomes that let the artifact into a lock exist: a failed or
/// incomplete check makes `sync` fail instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LockedIntegrity {
    /// Every declared checksum matched the decoded bytes, and at least one
    /// uses a non-weak algorithm. Integrity only, not authenticity.
    Verified,
    /// Every declared checksum matched, but all of them are weak (SHA-1) and
    /// the caller accepted that explicitly.
    VerifiedWeakOnly,
    /// The format declares no checksum, so nothing was checked.
    NotDeclared,
}

impl LockedIntegrity {
    /// The name used in the lock file.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "Verified",
            Self::VerifiedWeakOnly => "VerifiedWeakOnly",
            Self::NotDeclared => "NotDeclared",
        }
    }
}

/// A declared checksum as recorded in a lock.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct LockedChecksum {
    /// Algorithm name as declared (for example `SHA-256`).
    pub algorithm: String,
    /// Lowercase hex digest as declared.
    pub value: String,
}

impl LockedChecksum {
    /// Record one declared checksum.
    #[must_use]
    pub fn new(algorithm: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            algorithm: algorithm.into(),
            value: value.into(),
        }
    }
}

/// One format of one artifact revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct LockedArtifact {
    /// Artifact UUID.
    pub uuid: String,
    /// Revision number.
    pub revision: u64,
    /// Artifact type as the server published it (for example `BOM`).
    #[serde(rename = "type")]
    pub artifact_type: String,
    /// Media type of the format.
    pub media_type: String,
    /// Decoded size in bytes, as counted while downloading.
    pub size: u64,
    /// Declared checksums, sorted.
    pub checksums: Vec<LockedChecksum>,
    /// Integrity outcome when the lock was written.
    pub integrity: LockedIntegrity,
    /// When the bytes were fetched (RFC 3339 UTC with `Z`). Ignored by
    /// [`diff`], because it changes on every run.
    pub fetched_at: String,
}

impl LockedArtifact {
    /// Build an entry. Checksums are sorted by [`LockFile::normalize`].
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        uuid: impl Into<String>,
        revision: u64,
        artifact_type: impl Into<String>,
        media_type: impl Into<String>,
        size: u64,
        checksums: Vec<LockedChecksum>,
        integrity: LockedIntegrity,
        fetched_at: impl Into<String>,
    ) -> Self {
        Self {
            uuid: uuid.into(),
            revision,
            artifact_type: artifact_type.into(),
            media_type: media_type.into(),
            size,
            checksums,
            integrity,
            fetched_at: fetched_at.into(),
        }
    }

    fn key(&self) -> (&str, &str) {
        (self.uuid.as_str(), self.media_type.as_str())
    }
}

/// A lock file, `schemaVersion` 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct LockFile {
    /// Always [`LOCK_SCHEMA_VERSION`].
    pub schema_version: u64,
    /// The TEI that was resolved, as given.
    pub tei: String,
    /// The API root that served the release, without userinfo, query or fragment.
    pub server: String,
    /// The advertised API version that was used.
    pub api_version: String,
    /// The TEA specification commit this client is pinned to.
    pub spec_commit: String,
    /// Product release UUID.
    pub product_release: String,
    /// Collection version that was locked.
    pub collection_version: u64,
    /// Artifacts, sorted by UUID then media type.
    pub artifacts: Vec<LockedArtifact>,
}

impl LockFile {
    /// Start a lock with no artifacts.
    #[must_use]
    pub fn new(
        tei: impl Into<String>,
        server: impl Into<String>,
        api_version: impl Into<String>,
        spec_commit: impl Into<String>,
        product_release: impl Into<String>,
        collection_version: u64,
    ) -> Self {
        Self {
            schema_version: LOCK_SCHEMA_VERSION,
            tei: tei.into(),
            server: server.into(),
            api_version: api_version.into(),
            spec_commit: spec_commit.into(),
            product_release: product_release.into(),
            collection_version,
            artifacts: Vec::new(),
        }
    }

    /// Put artifacts and checksums into their canonical order.
    pub fn normalize(&mut self) {
        for artifact in &mut self.artifacts {
            artifact.checksums.sort();
        }
        self.artifacts.sort_by(|a, b| {
            (a.uuid.as_str(), a.media_type.as_str(), a.revision).cmp(&(
                b.uuid.as_str(),
                b.media_type.as_str(),
                b.revision,
            ))
        });
    }

    /// Check the lock is one this crate would write.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported `schemaVersion`, a `server` that
    /// could carry a secret, an identifier or timestamp that does not parse, a
    /// checksum that is not valid, or two entries for the same artifact and
    /// media type.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.schema_version != LOCK_SCHEMA_VERSION {
            return Err(ModelError::Invalid(format!(
                "unsupported lock schemaVersion {}",
                self.schema_version
            )));
        }
        check_server(&self.server)?;
        ProductReleaseId::parse(&self.product_release)?;
        if self.collection_version < 1 {
            return Err(ModelError::Invalid(
                "collectionVersion starts at 1".to_owned(),
            ));
        }
        let mut seen = BTreeMap::new();
        for artifact in &self.artifacts {
            ArtifactId::parse(&artifact.uuid)?;
            TeaTimestamp::parse(&artifact.fetched_at)?;
            if artifact.revision < 1 {
                return Err(ModelError::Invalid("artifact revision starts at 1".into()));
            }
            for checksum in &artifact.checksums {
                Checksum::new(
                    crate::enums::ChecksumAlgorithm::from_wire(&checksum.algorithm),
                    &checksum.value,
                )?;
            }
            if seen.insert(artifact.key(), ()).is_some() {
                return Err(ModelError::Invalid(format!(
                    "two lock entries for artifact {} and media type {}",
                    artifact.uuid, artifact.media_type
                )));
            }
        }
        Ok(())
    }

    /// Serialise in the canonical form: validated, sorted, two-space indent,
    /// camelCase keys in a fixed order, one trailing newline. The same content
    /// always yields the same bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when [`LockFile::validate`] fails.
    pub fn to_json(&self) -> Result<String, ModelError> {
        let mut canonical = self.clone();
        canonical.normalize();
        canonical.validate()?;
        let mut text = serde_json::to_string_pretty(&canonical)
            .map_err(|err| ModelError::Invalid(format!("lock serialisation failed: {err}")))?;
        text.push('\n');
        Ok(text)
    }

    /// Parse and validate a lock file.
    ///
    /// Unknown fields are rejected: the format is ours and version-gated, so a
    /// field this reader does not know means a newer `schemaVersion`, and
    /// silently dropping it on the next write would lose data.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON, an unsupported `schemaVersion`, an
    /// unknown field, or a lock that fails [`LockFile::validate`].
    pub fn parse(bytes: &[u8]) -> Result<Self, ModelError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Probe {
            schema_version: Option<u64>,
        }
        let probe: Probe = serde_json::from_slice(bytes)
            .map_err(|err| ModelError::Invalid(format!("lock file is not valid JSON: {err}")))?;
        match probe.schema_version {
            Some(LOCK_SCHEMA_VERSION) => {}
            Some(other) => {
                return Err(ModelError::Invalid(format!(
                    "unsupported lock schemaVersion {other}"
                )))
            }
            None => {
                return Err(ModelError::Invalid(
                    "lock file has no schemaVersion".to_owned(),
                ))
            }
        }
        let mut lock: Self = serde_json::from_slice(bytes)
            .map_err(|err| ModelError::Invalid(format!("lock file is not valid: {err}")))?;
        lock.validate()?;
        lock.normalize();
        Ok(lock)
    }
}

fn check_server(server: &str) -> Result<(), ModelError> {
    let url = url::Url::parse(server)
        .map_err(|_| ModelError::Invalid("lock server is not a URL".to_owned()))?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ModelError::Invalid(
            "lock server must not carry userinfo, a query or a fragment".to_owned(),
        ));
    }
    Ok(())
}

/// A change to one scalar of the lock header or of an entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct FieldChange {
    /// Field name as in the lock file.
    pub field: String,
    /// Value in the old lock.
    pub old: String,
    /// Value in the new lock.
    pub new: String,
}

/// An entry present in both locks whose content differs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ChangedArtifact {
    /// Artifact UUID.
    pub uuid: String,
    /// Media type of the format.
    pub media_type: String,
    /// What differs.
    pub fields: Vec<FieldChange>,
}

/// Differences between two locks. `fetchedAt` is ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct LockDiff {
    /// Header fields that differ.
    pub header: Vec<FieldChange>,
    /// Entries only in the new lock.
    pub added: Vec<LockedArtifact>,
    /// Entries only in the old lock.
    pub removed: Vec<LockedArtifact>,
    /// Entries in both whose content differs.
    pub modified: Vec<ChangedArtifact>,
}

impl LockDiff {
    /// Whether the two locks are equal apart from `fetchedAt`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.header.is_empty()
            && self.added.is_empty()
            && self.removed.is_empty()
            && self.modified.is_empty()
    }
}

fn change(out: &mut Vec<FieldChange>, field: &str, old: String, new: String) {
    if old != new {
        out.push(FieldChange {
            field: field.to_owned(),
            old,
            new,
        });
    }
}

fn checksums_text(checksums: &[LockedChecksum]) -> String {
    let mut sorted: Vec<&LockedChecksum> = checksums.iter().collect();
    sorted.sort();
    sorted
        .iter()
        .map(|checksum| format!("{}:{}", checksum.algorithm, checksum.value))
        .collect::<Vec<_>>()
        .join(",")
}

/// Compare two locks. Neither is modified; entries are matched by artifact
/// UUID and media type, so a new revision shows as a change, not as a removal
/// plus an addition.
#[must_use]
pub fn diff(old: &LockFile, new: &LockFile) -> LockDiff {
    let mut header = Vec::new();
    change(&mut header, "tei", old.tei.clone(), new.tei.clone());
    change(
        &mut header,
        "server",
        old.server.clone(),
        new.server.clone(),
    );
    change(
        &mut header,
        "apiVersion",
        old.api_version.clone(),
        new.api_version.clone(),
    );
    change(
        &mut header,
        "specCommit",
        old.spec_commit.clone(),
        new.spec_commit.clone(),
    );
    change(
        &mut header,
        "productRelease",
        old.product_release.clone(),
        new.product_release.clone(),
    );
    change(
        &mut header,
        "collectionVersion",
        old.collection_version.to_string(),
        new.collection_version.to_string(),
    );

    let old_by_key: BTreeMap<_, _> = old.artifacts.iter().map(|a| (a.key(), a)).collect();
    let new_by_key: BTreeMap<_, _> = new.artifacts.iter().map(|a| (a.key(), a)).collect();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    for (key, entry) in &new_by_key {
        match old_by_key.get(key) {
            None => added.push((*entry).clone()),
            Some(before) => {
                let mut fields = Vec::new();
                change(
                    &mut fields,
                    "revision",
                    before.revision.to_string(),
                    entry.revision.to_string(),
                );
                change(
                    &mut fields,
                    "type",
                    before.artifact_type.clone(),
                    entry.artifact_type.clone(),
                );
                change(
                    &mut fields,
                    "size",
                    before.size.to_string(),
                    entry.size.to_string(),
                );
                change(
                    &mut fields,
                    "checksums",
                    checksums_text(&before.checksums),
                    checksums_text(&entry.checksums),
                );
                change(
                    &mut fields,
                    "integrity",
                    before.integrity.as_str().to_owned(),
                    entry.integrity.as_str().to_owned(),
                );
                if !fields.is_empty() {
                    modified.push(ChangedArtifact {
                        uuid: key.0.to_owned(),
                        media_type: key.1.to_owned(),
                        fields,
                    });
                }
            }
        }
    }
    for (key, entry) in &old_by_key {
        if !new_by_key.contains_key(key) {
            removed.push((*entry).clone());
        }
    }
    LockDiff {
        header,
        added,
        removed,
        modified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REL: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
    const A1: &str = "11111111-1111-4111-8111-111111111111";
    const A2: &str = "22222222-2222-4222-8222-222222222222";
    const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn entry(uuid: &str, media: &str) -> LockedArtifact {
        LockedArtifact::new(
            uuid,
            1,
            "BOM",
            media,
            3,
            vec![LockedChecksum::new("SHA-256", SHA256_ABC)],
            LockedIntegrity::Verified,
            "2026-11-08T00:00:00Z",
        )
    }

    fn lock(artifacts: Vec<LockedArtifact>) -> LockFile {
        let mut lock = LockFile::new(
            "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
            "https://tea.example.com/",
            "1.0.0",
            "962a0228af1bfcc1232ac3f79239317ff33fdb04",
            REL,
            2,
        );
        lock.artifacts = artifacts;
        lock
    }

    #[test]
    fn serialisation_is_canonical_and_round_trips() {
        let a = lock(vec![entry(A2, "text/plain"), entry(A1, "application/json")]);
        let b = lock(vec![entry(A1, "application/json"), entry(A2, "text/plain")]);
        let text = a.to_json().unwrap();
        assert_eq!(text, b.to_json().unwrap(), "input order must not matter");
        assert!(text.ends_with("}\n"));
        let parsed = LockFile::parse(text.as_bytes()).unwrap();
        assert_eq!(parsed.to_json().unwrap(), text);
        assert_eq!(parsed.artifacts[0].uuid, A1);
        // The documented key names.
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "apiVersion",
                "artifacts",
                "collectionVersion",
                "productRelease",
                "schemaVersion",
                "server",
                "specCommit",
                "tei"
            ]
        );
        let mut entry_keys: Vec<_> = value["artifacts"][0]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        entry_keys.sort();
        assert_eq!(
            entry_keys,
            [
                "checksums",
                "fetchedAt",
                "integrity",
                "mediaType",
                "revision",
                "size",
                "type",
                "uuid"
            ]
        );
    }

    #[test]
    fn unsupported_version_unknown_fields_and_garbage_are_rejected() {
        let good = lock(vec![entry(A1, "text/plain")]).to_json().unwrap();
        let v2 = good.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2");
        assert!(LockFile::parse(v2.as_bytes()).is_err());
        let extra = good.replacen("\"tei\"", "\"extra\": 1,\n  \"tei\"", 1);
        assert!(LockFile::parse(extra.as_bytes()).is_err());
        let no_version = good.replace("\"schemaVersion\": 1,", "");
        assert!(LockFile::parse(no_version.as_bytes()).is_err());
        assert!(LockFile::parse(b"not json").is_err());
        assert!(LockFile::parse(b"[]").is_err());
        assert!(LockFile::parse(b"").is_err());
    }

    #[test]
    fn a_server_that_could_hold_a_secret_is_rejected() {
        for server in [
            "https://user:pw@tea.example.com/",
            "https://tea.example.com/?token=abc",
            "https://tea.example.com/#frag",
            "not a url",
        ] {
            let mut bad = lock(vec![]);
            bad.server = server.to_owned();
            assert!(bad.validate().is_err(), "{server}");
            assert!(bad.to_json().is_err(), "{server}");
        }
    }

    #[test]
    fn bad_identifiers_checksums_timestamps_and_duplicates_are_rejected() {
        let mut bad_uuid = lock(vec![entry("not-a-uuid", "text/plain")]);
        assert!(bad_uuid.validate().is_err());
        bad_uuid.artifacts[0].uuid = A1.into();
        assert!(bad_uuid.validate().is_ok());

        let mut bad_time = lock(vec![entry(A1, "text/plain")]);
        bad_time.artifacts[0].fetched_at = "yesterday".into();
        assert!(bad_time.validate().is_err());

        let mut bad_sum = lock(vec![entry(A1, "text/plain")]);
        bad_sum.artifacts[0].checksums = vec![LockedChecksum::new("SHA-256", "ABC")];
        assert!(bad_sum.validate().is_err());

        let dup = lock(vec![entry(A1, "text/plain"), entry(A1, "text/plain")]);
        assert!(dup.validate().is_err());

        let mut bad_collection = lock(vec![]);
        bad_collection.collection_version = 0;
        assert!(bad_collection.validate().is_err());
    }

    #[test]
    fn diff_reports_added_removed_changed_and_header_but_not_fetched_at() {
        let old = lock(vec![entry(A1, "text/plain"), entry(A2, "text/plain")]);
        assert!(diff(&old, &old).is_empty());

        let mut later = old.clone();
        for artifact in &mut later.artifacts {
            artifact.fetched_at = "2030-01-01T00:00:00Z".into();
        }
        assert!(diff(&old, &later).is_empty(), "fetchedAt must be ignored");

        let mut new = old.clone();
        new.collection_version = 3;
        new.artifacts.remove(1);
        new.artifacts.push(entry(A2, "application/json"));
        new.artifacts[0].revision = 2;
        new.artifacts[0].size = 99;
        new.artifacts[0].checksums = vec![LockedChecksum::new("SHA-256", "0".repeat(64))];
        let delta = diff(&old, &new);
        assert_eq!(delta.header.len(), 1);
        assert_eq!(delta.header[0].field, "collectionVersion");
        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.added[0].media_type, "application/json");
        assert_eq!(delta.removed.len(), 1);
        assert_eq!(delta.removed[0].uuid, A2);
        assert_eq!(delta.modified.len(), 1);
        let fields: Vec<_> = delta.modified[0]
            .fields
            .iter()
            .map(|f| f.field.as_str())
            .collect();
        assert_eq!(fields, ["revision", "size", "checksums"]);
        assert!(!delta.is_empty());
    }

    #[test]
    fn integrity_names_are_stable() {
        assert_eq!(LockedIntegrity::Verified.as_str(), "Verified");
        assert_eq!(
            LockedIntegrity::VerifiedWeakOnly.as_str(),
            "VerifiedWeakOnly"
        );
        assert_eq!(LockedIntegrity::NotDeclared.as_str(), "NotDeclared");
    }
}
