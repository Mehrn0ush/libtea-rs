//! Opt-in persistent HTTP cache (`docs/cache.md`).
//!
//! *Experimental until 0.9.* Enabled with [`crate::ClientBuilder::cache_dir`] or
//! [`crate::ClientBuilder::cache`]. Off by default.
//!
//! What it is: a per-user directory of response bodies, used to send
//! conditional `GET`s (`ETag` / `Last-Modified`) and, with
//! [`crate::ClientBuilder::offline`], to answer without the network.
//!
//! What it is not: a trust boundary. An entry is checked for truncation and
//! corruption (length and SHA-256), but anyone who can write the directory can
//! write a consistent entry. So the directory is created private (mode 0700,
//! files 0600 on Unix), a symbolic link or a directory open to other users is
//! refused, artifact bytes are checked against their declared checksums every
//! time they are used, and an entry that fails any check is treated as absent
//! and removed. Writes are atomic (temporary file, then rename) but are not
//! `fsync`ed; a crash can lose an entry, never leave a half-written one in use.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use libtea_core::escape_control_chars;
use libtea_core::http_cache::{
    decode_entry, encode_entry, is_cache_key, CacheEntryMeta, EntryBuilder,
};
use tokio::io::AsyncWriteExt;

use crate::error::ClientError;

/// Largest metadata block plus framing an entry file adds to its body.
const FRAMING_SLACK: u64 = 80 * 1024;
const ENTRY_DIR: &str = "v1";

/// Cache settings. Defaults are conservative; every limit is a ceiling.
///
/// *Experimental.*
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CacheConfig {
    /// The cache directory. Created private if missing.
    pub dir: PathBuf,
    /// Total bytes of entries kept; the oldest are removed past this.
    pub max_total_bytes: u64,
    /// Largest body stored. A larger response is simply not cached.
    pub max_entry_bytes: u64,
    /// Oldest an entry may be, counted from its last successful revalidation.
    /// An older entry is treated as absent, offline mode included.
    pub max_age: Duration,
    /// Also store responses to requests that carried credentials, and
    /// `Cache-Control: private` responses. Off by default.
    pub authenticated: bool,
}

impl CacheConfig {
    /// Defaults: 512 MiB total, 64 MiB per entry, 7 days, no authenticated
    /// responses.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            max_total_bytes: 512 * 1024 * 1024,
            max_entry_bytes: 64 * 1024 * 1024,
            max_age: Duration::from_secs(7 * 24 * 60 * 60),
            authenticated: false,
        }
    }

    /// Set [`Self::max_total_bytes`].
    #[must_use]
    pub fn with_max_total_bytes(mut self, value: u64) -> Self {
        self.max_total_bytes = value;
        self
    }

    /// Set [`Self::max_entry_bytes`].
    #[must_use]
    pub fn with_max_entry_bytes(mut self, value: u64) -> Self {
        self.max_entry_bytes = value;
        self
    }

    /// Set [`Self::max_age`].
    #[must_use]
    pub fn with_max_age(mut self, value: Duration) -> Self {
        self.max_age = value;
        self
    }

    /// Set [`Self::authenticated`].
    #[must_use]
    pub fn with_authenticated(mut self, value: bool) -> Self {
        self.authenticated = value;
        self
    }
}

pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn io_error(what: &str, err: &std::io::Error) -> ClientError {
    ClientError::Io(escape_control_chars(&format!("{what}: {}", err.kind())))
}

fn refuse(path: &Path, why: &str) -> ClientError {
    ClientError::Io(escape_control_chars(&format!(
        "cache directory {}: {why}",
        path.display()
    )))
}

/// Check (and optionally create) a directory the cache will use. Returns
/// whether it exists. Refuses a symbolic link, a non-directory, and on Unix a
/// directory that group or others can access.
fn private_dir(path: &Path, create: bool) -> Result<bool, ClientError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(refuse(path, "is a symbolic link; refusing to use it"))
        }
        Ok(meta) if !meta.is_dir() => Err(refuse(path, "is not a directory")),
        Ok(meta) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};
                // A private directory that someone else owns is theirs to
                // change; refuse it. The reference uid is that of a file this
                // process has just created, so no uid lookup is needed.
                if !owner_matches(meta.uid(), current_uid(Some(path))?) {
                    return Err(refuse(
                        path,
                        "is owned by another user; use a directory you own",
                    ));
                }
                let mode = meta.permissions().mode() & 0o777;
                if mode & 0o077 != 0 {
                    return Err(refuse(
                        path,
                        &format!(
                            "is accessible by other users (mode {mode:o}); run chmod 700 on it"
                        ),
                    ));
                }
            }
            #[cfg(not(unix))]
            let _ = meta;
            Ok(true)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && create => {
            let mut builder = std::fs::DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder
                .create(path)
                .map_err(|e| io_error("creating the cache directory", &e))?;
            // Look again: what we created must be what we meant to create.
            private_dir(path, false)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(io_error("inspecting the cache directory", &err)),
    }
}

/// Whether a directory owned by `dir_uid` belongs to the user running the
/// process, whose files get `own_uid`.
#[cfg(unix)]
pub(crate) fn owner_matches(dir_uid: u32, own_uid: u32) -> bool {
    dir_uid == own_uid
}

/// The uid this process creates files as: the owner of a file created now,
/// in the temporary directory or, if that is not writable, in `fallback`.
#[cfg(unix)]
pub(crate) fn current_uid(fallback: Option<&Path>) -> Result<u32, ClientError> {
    use std::os::unix::fs::MetadataExt;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let name = format!(
        ".libtea-uid-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let mut last = None;
    for dir in std::iter::once(std::env::temp_dir()).chain(fallback.map(Path::to_path_buf)) {
        let probe = dir.join(&name);
        match open_private(&probe) {
            Ok(file) => {
                let uid = file
                    .metadata()
                    .map_err(|e| io_error("checking directory ownership", &e))?
                    .uid();
                drop(file);
                let _ = std::fs::remove_file(&probe);
                return Ok(uid);
            }
            Err(err) => last = Some(err),
        }
    }
    Err(match last {
        Some(err) => io_error("checking directory ownership", &err),
        None => ClientError::Io("checking directory ownership".into()),
    })
}

pub(crate) struct Cache {
    pub(crate) config: CacheConfig,
    entries: PathBuf,
    counter: AtomicU64,
    pub(crate) offline: bool,
}

impl Cache {
    pub(crate) fn open(config: CacheConfig, offline: bool) -> Result<Self, ClientError> {
        private_dir(&config.dir, true)?;
        let entries = config.dir.join(ENTRY_DIR);
        private_dir(&entries, true)?;
        Ok(Self {
            config,
            entries,
            counter: AtomicU64::new(0),
            offline,
        })
    }

    fn entry_path(&self, key: &str) -> PathBuf {
        debug_assert!(is_cache_key(key));
        self.entries.join(format!("{key}.entry"))
    }

    fn temp_path(&self, key: &str) -> PathBuf {
        self.entries.join(format!(
            ".{key}.{}-{}.tmp",
            std::process::id(),
            self.counter.fetch_add(1, Ordering::Relaxed)
        ))
    }

    /// Read and check one entry; anything wrong removes it and returns `None`.
    pub(crate) fn load_blocking(&self, key: &str) -> Option<(CacheEntryMeta, Vec<u8>)> {
        if !is_cache_key(key) {
            return None;
        }
        let path = self.entry_path(key);
        let meta = std::fs::symlink_metadata(&path).ok()?;
        let remove = |path: &Path| {
            let _ = std::fs::remove_file(path);
        };
        if !meta.file_type().is_file() || meta.len() > self.config.max_entry_bytes + FRAMING_SLACK {
            remove(&path);
            return None;
        }
        let bytes = read_regular(&path, self.config.max_entry_bytes + FRAMING_SLACK).ok()?;
        let max_body = usize::try_from(self.config.max_entry_bytes).unwrap_or(usize::MAX);
        match decode_entry(&bytes, key, max_body) {
            Ok((entry, body)) => {
                let now = now_secs();
                let age = now.saturating_sub(entry.validated_at);
                if entry.validated_at > now.saturating_add(24 * 60 * 60)
                    || age > self.config.max_age.as_secs()
                {
                    remove(&path);
                    return None;
                }
                Some((entry, body))
            }
            Err(error) => {
                tracing::debug!(%error, "cache entry refused and removed");
                remove(&path);
                None
            }
        }
    }

    pub(crate) async fn load(self: &Arc<Self>, key: &str) -> Option<(CacheEntryMeta, Vec<u8>)> {
        let cache = Arc::clone(self);
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || cache.load_blocking(&key))
            .await
            .ok()
            .flatten()
    }

    /// Store one entry atomically. Never fails the caller: a problem is logged.
    pub(crate) fn store_blocking(&self, key: &str, meta: CacheEntryMeta, body: &[u8]) {
        if !is_cache_key(key) || body.len() as u64 > self.config.max_entry_bytes {
            return;
        }
        let Ok(bytes) = encode_entry(meta, body) else {
            return;
        };
        let tmp = self.temp_path(key);
        let result = open_private(&tmp)
            .and_then(|mut file| file.write_all(&bytes))
            .and_then(|()| std::fs::rename(&tmp, self.entry_path(key)));
        match result {
            Ok(()) => self.prune_blocking(),
            Err(error) => {
                tracing::debug!(kind = ?error.kind(), "cache entry not written");
                let _ = std::fs::remove_file(&tmp);
            }
        }
    }

    pub(crate) async fn store(self: &Arc<Self>, key: &str, meta: CacheEntryMeta, body: Vec<u8>) {
        let cache = Arc::clone(self);
        let key = key.to_owned();
        let _ = tokio::task::spawn_blocking(move || cache.store_blocking(&key, meta, &body)).await;
    }

    pub(crate) fn remove_blocking(&self, key: &str) {
        if is_cache_key(key) {
            let _ = std::fs::remove_file(self.entry_path(key));
        }
    }

    pub(crate) async fn remove(self: &Arc<Self>, key: &str) {
        let cache = Arc::clone(self);
        let key = key.to_owned();
        let _ = tokio::task::spawn_blocking(move || cache.remove_blocking(&key)).await;
    }

    /// Drop the oldest entries until the total is within the limit, and stale
    /// temporary files left by a crash.
    pub(crate) fn prune_blocking(&self) {
        let Ok(dir) = std::fs::read_dir(&self.entries) else {
            return;
        };
        let now = SystemTime::now();
        let mut entries: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
        let mut total = 0u64;
        for item in dir.flatten() {
            let path = item.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            let name = item.file_name();
            let name = name.to_string_lossy();
            if is_cache_temp(&name) {
                let old = meta
                    .modified()
                    .ok()
                    .and_then(|m| now.duration_since(m).ok())
                    .is_some_and(|age| age > Duration::from_secs(3600));
                if old {
                    let _ = std::fs::remove_file(&path);
                }
                continue;
            }
            if !meta.file_type().is_file() || !name.ends_with(".entry") {
                continue;
            }
            total = total.saturating_add(meta.len());
            entries.push((meta.modified().unwrap_or(UNIX_EPOCH), meta.len(), path));
        }
        if total <= self.config.max_total_bytes {
            return;
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.cmp(&b.2)));
        for (_, size, path) in entries {
            if total <= self.config.max_total_bytes {
                break;
            }
            if std::fs::remove_file(&path).is_ok() {
                total = total.saturating_sub(size);
            }
        }
    }

    /// Begin writing an entry whose body is still arriving. `None` when the
    /// temporary file cannot be created.
    pub(crate) async fn start_tee(self: &Arc<Self>, key: &str) -> Option<CacheTee> {
        if !is_cache_key(key) {
            return None;
        }
        let tmp = self.temp_path(key);
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&tmp).await.ok()?;
        if file.write_all(EntryBuilder::prefix()).await.is_err() {
            let _ = std::fs::remove_file(&tmp);
            return None;
        }
        Some(CacheTee {
            cache: Arc::clone(self),
            key: key.to_owned(),
            file: Some(file),
            tmp,
            builder: EntryBuilder::new(),
        })
    }
}

/// Open `path` only if it is a regular file, and keep that true while opening.
///
/// The path is inspected with `lstat`, opened, and inspected again; the opened
/// file must be the regular file the path names both times (same device and
/// inode). A symbolic link swapped in between the checks is refused instead of
/// followed, which `lstat`-then-read alone would not do. `between` runs after
/// the first inspection and exists so a test can make that swap happen.
#[cfg(unix)]
fn open_regular_with(path: &Path, between: impl FnOnce()) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::MetadataExt;
    let invalid =
        |what: &str| std::io::Error::new(std::io::ErrorKind::InvalidInput, what.to_owned());
    let before = std::fs::symlink_metadata(path)?;
    if !before.file_type().is_file() {
        return Err(invalid("not a regular file"));
    }
    between();
    let file = std::fs::File::open(path)?;
    let opened = file.metadata()?;
    let after = std::fs::symlink_metadata(path)?;
    if !after.file_type().is_file()
        || !opened.is_file()
        || (opened.dev(), opened.ino()) != (after.dev(), after.ino())
        || (opened.dev(), opened.ino()) != (before.dev(), before.ino())
    {
        return Err(invalid("the path changed while it was opened"));
    }
    Ok(file)
}

/// Read at most `limit` bytes of a regular file (see [`open_regular_with`]);
/// a longer file is an error.
fn read_regular(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    #[cfg(unix)]
    let file = open_regular_with(path, || {})?;
    #[cfg(not(unix))]
    let file = {
        if !std::fs::symlink_metadata(path)?.file_type().is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        std::fs::File::open(path)?
    };
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "entry is larger than the limit",
        ));
    }
    Ok(bytes)
}

fn open_private(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// An entry being written while the body streams past. The file is removed
/// unless [`CacheTee::finish`] commits it, so a failed or cancelled download
/// leaves nothing behind.
pub(crate) struct CacheTee {
    cache: Arc<Cache>,
    key: String,
    file: Option<tokio::fs::File>,
    tmp: PathBuf,
    builder: EntryBuilder,
}

impl CacheTee {
    /// Add body bytes. Returns `false` once the entry is abandoned (too large
    /// or a write failed); later calls are no-ops.
    pub(crate) async fn write(&mut self, chunk: &[u8]) -> bool {
        let Some(file) = self.file.as_mut() else {
            return false;
        };
        if self.builder.len().saturating_add(chunk.len() as u64) > self.cache.config.max_entry_bytes
            || file.write_all(chunk).await.is_err()
        {
            self.abandon();
            return false;
        }
        self.builder.update(chunk);
        true
    }

    fn abandon(&mut self) {
        self.file = None;
        let _ = std::fs::remove_file(&self.tmp);
    }

    /// Commit the entry. Call only after the body passed every check.
    pub(crate) async fn finish(mut self, meta: CacheEntryMeta) {
        let Some(mut file) = self.file.take() else {
            return;
        };
        let builder = std::mem::take(&mut self.builder);
        let Ok(trailer) = builder.trailer(meta) else {
            self.abandon();
            return;
        };
        if file.write_all(&trailer).await.is_err() || file.flush().await.is_err() {
            drop(file);
            let _ = std::fs::remove_file(&self.tmp);
            return;
        }
        drop(file);
        let cache = Arc::clone(&self.cache);
        let tmp = self.tmp.clone();
        let final_path = cache.entry_path(&self.key);
        let committed = tokio::task::spawn_blocking(move || {
            let ok = std::fs::rename(&tmp, final_path).is_ok();
            if ok {
                cache.prune_blocking();
            } else {
                let _ = std::fs::remove_file(&tmp);
            }
            ok
        })
        .await
        .unwrap_or(false);
        if committed {
            // The temporary file no longer exists; nothing for Drop to remove.
            self.tmp = PathBuf::new();
        }
    }
}

impl Drop for CacheTee {
    fn drop(&mut self) {
        if !self.tmp.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.tmp);
        }
    }
}

/// What `libtea cache stat` reports.
///
/// *Experimental.*
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CacheStats {
    /// The directory inspected.
    pub dir: PathBuf,
    /// Whether it exists.
    pub exists: bool,
    /// Entry files.
    pub entries: u64,
    /// Bytes in entry files.
    pub bytes: u64,
    /// Files, directories and links that are not entries.
    pub foreign: u64,
    /// Seconds since the oldest entry file was written (by file time).
    pub oldest_age_secs: Option<u64>,
    /// Seconds since the newest entry file was written (by file time).
    pub newest_age_secs: Option<u64>,
}

fn entry_files(dir: &Path) -> Result<Option<(PathBuf, std::fs::ReadDir)>, ClientError> {
    if !private_dir(dir, false)? {
        return Ok(None);
    }
    let entries = dir.join(ENTRY_DIR);
    if !private_dir(&entries, false)? {
        return Ok(None);
    }
    let read =
        std::fs::read_dir(&entries).map_err(|e| io_error("reading the cache directory", &e))?;
    Ok(Some((entries, read)))
}

/// Summarise a cache directory without changing it.
///
/// # Errors
///
/// Returns an error when the directory is a symbolic link, is open to other
/// users, or cannot be read. A directory that does not exist is not an error.
pub fn cache_stat(dir: &Path) -> Result<CacheStats, ClientError> {
    let mut stats = CacheStats {
        dir: dir.to_path_buf(),
        exists: false,
        entries: 0,
        bytes: 0,
        foreign: 0,
        oldest_age_secs: None,
        newest_age_secs: None,
    };
    let Some((_, read)) = entry_files(dir)? else {
        return Ok(stats);
    };
    stats.exists = true;
    let now = SystemTime::now();
    for item in read.flatten() {
        let name = item.file_name();
        let name = name.to_string_lossy().into_owned();
        let Ok(meta) = std::fs::symlink_metadata(item.path()) else {
            continue;
        };
        let key = name.strip_suffix(".entry").unwrap_or("");
        if meta.file_type().is_file() && is_cache_key(key) {
            stats.entries += 1;
            stats.bytes = stats.bytes.saturating_add(meta.len());
            if let Some(age) = meta
                .modified()
                .ok()
                .and_then(|m| now.duration_since(m).ok())
            {
                let age = age.as_secs();
                stats.oldest_age_secs = Some(stats.oldest_age_secs.map_or(age, |o| o.max(age)));
                stats.newest_age_secs = Some(stats.newest_age_secs.map_or(age, |n| n.min(age)));
            }
        } else if !name.ends_with(".tmp") {
            stats.foreign += 1;
        }
    }
    Ok(stats)
}

/// A temporary file the cache itself writes: `.<64 hex key>.<pid>-<n>.tmp`.
/// Nothing else is a cache temporary file, whatever its name ends in.
fn is_cache_temp(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('.') else {
        return false;
    };
    let Some(rest) = rest.strip_suffix(".tmp") else {
        return false;
    };
    let Some((key, tail)) = rest.split_once('.') else {
        return false;
    };
    is_cache_key(key)
        && tail.split_once('-').is_some_and(|(pid, n)| {
            !pid.is_empty()
                && !n.is_empty()
                && pid.bytes().all(|b| b.is_ascii_digit())
                && n.bytes().all(|b| b.is_ascii_digit())
        })
}

/// Remove every cache entry (and stray temporary files) and return how many
/// files were removed. Only files this cache writes are touched: names that are
/// a cache key plus `.entry`, and `*.tmp`. Other files, and the directories
/// themselves, are left alone. A symbolic link with an entry's name is removed
/// as a link; its target is never followed.
///
/// # Errors
///
/// Returns an error when the directory is a symbolic link, is open to other
/// users, or cannot be read.
pub fn cache_clear(dir: &Path) -> Result<u64, ClientError> {
    let Some((entries, read)) = entry_files(dir)? else {
        return Ok(0);
    };
    let mut removed = 0;
    for item in read.flatten() {
        let name = item.file_name();
        let name = name.to_string_lossy().into_owned();
        let is_entry = name.strip_suffix(".entry").is_some_and(is_cache_key);
        let is_temp = is_cache_temp(&name);
        if !(is_entry || is_temp) {
            continue;
        }
        let path = entries.join(&name);
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if (meta.file_type().is_file() || meta.file_type().is_symlink())
            && std::fs::remove_file(&path).is_ok()
        {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("libtea-cache-unit-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ownership_is_compared_with_the_uid_of_a_file_just_created() {
        let dir = scratch("uid");
        let mine = current_uid(None).unwrap();
        let probe = dir.join("probe");
        std::fs::write(&probe, b"x").unwrap();
        use std::os::unix::fs::MetadataExt;
        assert_eq!(std::fs::metadata(&probe).unwrap().uid(), mine);
        assert_eq!(std::fs::metadata(&dir).unwrap().uid(), mine);
        assert!(owner_matches(mine, mine));
        assert!(
            !owner_matches(mine.wrapping_add(1), mine),
            "another uid must not match"
        );
        assert!(!owner_matches(0, 501) && !owner_matches(501, 0));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_regular_file_opens_and_a_symlink_never_does() {
        let dir = scratch("nofollow");
        let real = dir.join("real");
        std::fs::write(&real, b"entry").unwrap();
        assert_eq!(read_regular(&real, 100).unwrap(), b"entry");
        let link = dir.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(open_regular_with(&link, || {}).is_err());
        assert!(read_regular(&link, 100).is_err());
        assert!(
            read_regular(&dir, 100).is_err(),
            "a directory is not an entry"
        );
        assert!(read_regular(&real, 3).is_err(), "over the limit");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_symlink_swapped_in_after_the_check_is_refused_not_followed() {
        let dir = scratch("swap");
        let entry = dir.join("entry");
        let secret = dir.join("secret");
        std::fs::write(&entry, b"genuine").unwrap();
        std::fs::write(&secret, b"must not be read").unwrap();
        // Between the first lstat and the open, replace the file by a link.
        let swapped = open_regular_with(&entry, || {
            std::fs::remove_file(&entry).unwrap();
            std::os::unix::fs::symlink(&secret, &entry).unwrap();
        });
        assert!(swapped.is_err(), "the swapped-in link was followed");
        // And a plain replacement by another regular file is noticed too.
        let entry2 = dir.join("entry2");
        std::fs::write(&entry2, b"first").unwrap();
        let replaced = open_regular_with(&entry2, || {
            std::fs::remove_file(&entry2).unwrap();
            std::fs::write(&entry2, b"second").unwrap();
        });
        assert!(
            replaced.is_err(),
            "a different file than the one inspected was accepted"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_the_caches_own_temporary_files_are_temporary_files() {
        let key = "a".repeat(64);
        assert!(is_cache_temp(&format!(".{key}.123-4.tmp")));
        for other in [
            ".mine.tmp",
            ".notes.libtea.tmp",
            "build.tmp",
            &format!(".{key}.tmp"),
            &format!(".{key}.x-4.tmp"),
            &format!(".{key}.12-.tmp"),
            &format!(".{key}.12-4.tmp.bak"),
            &format!(".{}.1-2.tmp", "g".repeat(64)),
            &format!(".{}.1-2.tmp", "a".repeat(63)),
            &format!("{key}.1-2.tmp"),
        ] {
            assert!(!is_cache_temp(other), "{other}");
        }
    }
}
