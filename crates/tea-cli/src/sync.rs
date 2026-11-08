//! `libtea sync` and `libtea diff`: resolve a TEI, fetch and verify its
//! artifacts, and keep a lock file (`docs/lock-file.md`).
//!
//! Everything fails closed: if any artifact cannot be fetched and checked, no
//! lock is written and an existing lock is left as it was. A lock records
//! checksum integrity of decoded bytes at fetch time, not publisher
//! authenticity.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use libtea_client::FileDownloadOptions;
use libtea_core::{
    diff_locks, Artifact, IntegrityOutcome, LockDiff, LockFile, LockedArtifact, LockedChecksum,
    LockedIntegrity, ProductReleaseId, TeaTimestamp, Tei,
};
use url::Url;

use super::{
    check_output_target, clean, coded, configured_builder, exit, map_download_error, redact_url,
    usage, with_cache, CacheArgs, CredentialArgs,
};

/// A lock file larger than this is refused before it is read.
const MAX_LOCK_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) struct SyncOptions {
    pub tei: String,
    pub lock: PathBuf,
    pub check: bool,
    pub artifacts_dir: Option<PathBuf>,
    pub product_release: Option<String>,
    pub collection_version: Option<u64>,
    pub fetched_at: Option<String>,
    pub allow_weak_checksums: bool,
    pub allow_discovered_credentials: bool,
    pub well_known_url: Option<Url>,
    pub allow_origin: Option<Url>,
    pub credentials: CredentialArgs,
    pub cache: CacheArgs,
    pub json: bool,
}

pub(crate) async fn sync(options: SyncOptions) -> anyhow::Result<()> {
    let tei = Tei::parse(&options.tei)?;
    let fetched_at = match &options.fetched_at {
        Some(text) => TeaTimestamp::parse(text)?.as_str().to_owned(),
        None => now_rfc3339(),
    };
    // Read the existing lock first so a bad one fails before any network use.
    let existing = if options.lock.exists() || options.check {
        Some(read_lock(&options.lock)?)
    } else {
        None
    };

    let mut builder = with_cache(
        configured_builder(&options.credentials, options.allow_origin.clone())?,
        &options.cache,
    )?
    .allow_weak_checksums(options.allow_weak_checksums)
    .allow_discovered_credentials(options.allow_discovered_credentials);
    if let Some(url) = options.well_known_url.clone() {
        builder = builder.well_known_fetch_url(url);
    }
    let client = builder.build()?;
    let releases = client.discover(&tei).await?;
    let release = select_release(&releases, options.product_release.as_deref())?;
    let service = client.service(&release.root_url, &release.api_version)?;
    let collection = match options.collection_version {
        Some(version) => {
            service
                .get_product_release_collection(release.product_release, version)
                .await?
        }
        None => {
            service
                .get_latest_product_release_collection(release.product_release)
                .await?
        }
    };

    if let Some(dir) = &options.artifacts_dir {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let mut lock = LockFile::new(
        options.tei.clone(),
        redact_url(&release.root_url),
        release.api_version.clone(),
        libtea_core::spec::COMMIT,
        release.product_release.to_string(),
        collection.version,
    );
    for artifact in &collection.artifacts {
        for format in &artifact.formats {
            lock.artifacts.push(
                fetch_one(
                    &service,
                    artifact,
                    &format.media_type,
                    options.artifacts_dir.as_deref(),
                    &fetched_at,
                )
                .await?,
            );
        }
    }
    lock.normalize();
    let text = lock
        .to_json()
        .map_err(|err| coded(exit::FAILURE, format!("the lock could not be built: {err}")))?;

    if options.check {
        let old = existing.expect("--check reads the lock first");
        let delta = diff_locks(&old, &lock);
        print_diff(&delta, options.json)?;
        if delta.is_empty() {
            if !options.json {
                out!(
                    "lock is up to date ({} artifact formats)",
                    lock.artifacts.len()
                );
            }
            return Ok(());
        }
        return Err(coded(
            exit::CHANGES_DETECTED,
            "the server differs from the lock",
        ));
    }

    let previous = match &existing {
        None => "none",
        Some(old) if diff_locks(old, &lock).is_empty() => "unchanged",
        Some(_) => "changed",
    };
    // An unchanged lock keeps its bytes (and its old fetchedAt), so a re-sync
    // produces no diff in version control.
    let written = previous != "unchanged";
    if written {
        write_lock(&options.lock, &text)?;
    }
    if options.json {
        out!(
            "{}",
            serde_json::json!({
                "schema_version": 1,
                "lock": options.lock.display().to_string(),
                "written": written,
                "previous": previous,
                "tei": options.tei,
                "product_release": lock.product_release,
                "collection_version": lock.collection_version,
                "artifacts": lock.artifacts.len(),
            })
        );
    } else {
        out!(
            "{} {} ({} artifact formats, collection {}, product release {})",
            if written { "wrote" } else { "unchanged" },
            options.lock.display(),
            lock.artifacts.len(),
            lock.collection_version,
            lock.product_release
        );
        err!("checksum integrity of decoded bytes only; a lock is not publisher authenticity");
    }
    Ok(())
}

pub(crate) fn diff(old: &Path, new: &Path, json: bool) -> anyhow::Result<()> {
    let old = read_lock(old)?;
    let new = read_lock(new)?;
    let delta = diff_locks(&old, &new);
    print_diff(&delta, json)?;
    if delta.is_empty() {
        if !json {
            out!("no differences");
        }
        Ok(())
    } else {
        Err(coded(exit::CHANGES_DETECTED, "the locks differ"))
    }
}

fn select_release(
    releases: &[libtea_client::ResolvedRelease],
    wanted: Option<&str>,
) -> anyhow::Result<libtea_client::ResolvedRelease> {
    if let Some(wanted) = wanted {
        let id = ProductReleaseId::parse(wanted)?;
        return releases
            .iter()
            .find(|release| release.product_release == id)
            .cloned()
            .ok_or_else(|| {
                coded(
                    exit::NOT_FOUND,
                    "the TEI did not resolve to that product release",
                )
            });
    }
    match releases {
        [] => Err(coded(
            exit::NOT_FOUND,
            "the TEI resolved to no product release",
        )),
        [only] => Ok(only.clone()),
        many => Err(usage(format!(
            "the TEI resolved to {} product releases; pass --product-release",
            many.len()
        ))),
    }
}

async fn fetch_one(
    service: &libtea_client::Service,
    artifact: &Artifact,
    media_type: &str,
    dir: Option<&Path>,
    fetched_at: &str,
) -> anyhow::Result<LockedArtifact> {
    let download = match dir {
        Some(dir) => {
            let dest = dir.join(file_name(
                &artifact.uuid.to_string(),
                artifact.version,
                media_type,
            ));
            let options = FileDownloadOptions::new().with_overwrite(true);
            service
                .download_format_to_file(artifact, media_type, &dest, &options)
                .await
                .map(|result| result.download)
        }
        None => {
            // Nothing is kept; the bytes are only counted and hashed.
            let mut sink = tokio::io::sink();
            service
                .download_format_to_writer(artifact, media_type, &mut sink)
                .await
        }
    }
    .map_err(map_download_error)?;
    let integrity = match download.integrity.outcome {
        IntegrityOutcome::Verified if download.integrity.checks.iter().all(|c| c.weak) => {
            LockedIntegrity::VerifiedWeakOnly
        }
        IntegrityOutcome::Verified => LockedIntegrity::Verified,
        IntegrityOutcome::NotDeclared => LockedIntegrity::NotDeclared,
        // The client turns every other outcome into an error already.
        _ => {
            return Err(coded(
                exit::INTEGRITY,
                format!(
                    "decoded-byte checksum integrity was not established ({})",
                    super::outcome_name(&download.integrity.outcome)
                ),
            ))
        }
    };
    let declared = artifact
        .format(media_type)
        .and_then(|format| format.checksums.as_ref())
        .map(|list| {
            list.iter()
                .map(|c| LockedChecksum::new(c.algorithm().as_str(), c.hex_value()))
                .collect()
        })
        .unwrap_or_default();
    Ok(LockedArtifact::new(
        artifact.uuid.to_string(),
        artifact.version,
        artifact.artifact_type.as_str(),
        media_type,
        download.bytes_written,
        declared,
        integrity,
        fetched_at,
    ))
}

/// `<uuid>-r<revision>-<media type>` with every character outside
/// `[A-Za-z0-9.+-]` replaced by `_`. Never a name the server chose for a file.
fn file_name(uuid: &str, revision: u64, media_type: &str) -> String {
    let slug: String = media_type
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{uuid}-r{revision}-{slug}")
}

fn read_lock(path: &Path) -> anyhow::Result<LockFile> {
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(usage(format!(
                "lock file {} does not exist",
                path.display()
            )));
        }
        Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
    };
    if !meta.is_file() {
        return Err(usage(format!("{} is not a regular file", path.display())));
    }
    if meta.len() > MAX_LOCK_BYTES {
        return Err(usage(format!(
            "{} is larger than {MAX_LOCK_BYTES} bytes",
            path.display()
        )));
    }
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    LockFile::parse(&bytes).map_err(|err| usage(format!("{}: {err}", path.display())))
}

/// Replace the lock atomically: a finished temporary file in the same
/// directory is renamed over it. A symbolic link at the path is refused.
fn write_lock(path: &Path, text: &str) -> anyhow::Result<()> {
    check_output_target(path, true)?;
    let name = path
        .file_name()
        .ok_or_else(|| usage(format!("{} has no file name", path.display())))?;
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(".libtea-{}.tmp", std::process::id()));
    let temp = dir.join(temp_name);
    let result = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| {
            file.write_all(text.as_bytes())
                .and_then(|()| file.sync_all())
        })
        .and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.with_context(|| format!("writing {}", path.display()))
}

fn print_diff(delta: &LockDiff, json: bool) -> anyhow::Result<()> {
    if json {
        out!(
            "{}",
            serde_json::json!({
                "schema_version": 1,
                "has_changes": !delta.is_empty(),
                "header": delta.header,
                "added": delta.added,
                "removed": delta.removed,
                "modified": delta.modified,
            })
        );
        return Ok(());
    }
    for change in &delta.header {
        out!("~ {}: {} -> {}", change.field, change.old, change.new);
    }
    for entry in &delta.added {
        out!(
            "+ {} {} revision {}",
            entry.uuid,
            entry.media_type,
            entry.revision
        );
    }
    for entry in &delta.removed {
        out!(
            "- {} {} revision {}",
            entry.uuid,
            entry.media_type,
            entry.revision
        );
    }
    for entry in &delta.modified {
        let fields: Vec<String> = entry
            .fields
            .iter()
            .map(|change| format!("{} {} -> {}", change.field, change.old, change.new))
            .collect();
        out!(
            "~ {} {}: {}",
            entry.uuid,
            entry.media_type,
            fields.join("; ")
        );
    }
    Ok(())
}

/// The current time as RFC 3339 UTC with a `Z`.
fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    format_utc(secs)
}

/// Seconds since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ` (proleptic Gregorian,
/// civil-from-days after H. Hinnant).
fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_formatting_matches_known_instants() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_utc(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(format_utc(4_102_444_799), "2099-12-31T23:59:59Z");
        assert!(TeaTimestamp::parse(&format_utc(1_762_560_000)).is_ok());
        assert!(TeaTimestamp::parse(&now_rfc3339()).is_ok());
    }

    #[test]
    fn file_names_never_contain_a_path_separator_or_come_from_the_server() {
        let name = file_name("u", 2, "../../etc/passwd");
        assert!(!name.contains('/') && !name.contains('\\'));
        assert_eq!(name, "u-r2-.._.._etc_passwd");
        assert_eq!(
            file_name("u", 1, "application/vnd.cyclonedx+json"),
            "u-r1-application_vnd.cyclonedx+json"
        );
        assert!(!file_name("u", 1, "a\nb\u{1b}[0m").contains('\u{1b}'));
    }
}
