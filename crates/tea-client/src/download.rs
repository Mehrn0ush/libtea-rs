//! Streaming artifact download: hash while writing, release only after the check.
//!
//! *Experimental until 0.9.* The types and methods in this module may change in
//! a minor release.
//!
//! The whole-body methods on [`crate::Service`] hold the decoded artifact in
//! memory. The methods here write the decoded bytes to a sink as they arrive
//! and feed a [`libtea_core::ChecksumVerifier`] with the same bytes, so memory
//! use does not grow with the artifact. The same limits apply as for the
//! whole-body methods: [`crate::Limits::max_artifact_bytes`] caps the encoded
//! and the decoded size, redirects and destinations go through the same
//! policy, and only one content coding is accepted.
//!
//! **Verified means checksum integrity.** A declared checksum that matches the
//! decoded bytes is integrity of those bytes, not publisher authenticity.
//!
//! **Nothing is released as verified before the check completes.**
//!
//! - [`Service::download_format_to_file`] and its siblings write to a
//!   temporary file next to the destination, then rename it over the
//!   destination only after the stream ended cleanly and the checksums passed
//!   the client's policy. On any failure the temporary file is removed and the
//!   destination is untouched.
//! - [`Service::download_format_to_writer`] cannot take bytes back from a
//!   writer it does not own. The writer sees unverified bytes. Treat what it
//!   received as untrusted unless the call returned `Ok`, and discard it on
//!   `Err`.
//!
//! **Resume is not supported.** There is no `Range` request. A `Range` over a
//! content-coded representation does not line up with the decoded bytes that
//! the checksum covers, a partial file would have to be re-read to rebuild the
//! hash state, and TEA 1.0.0 does not define range semantics for artifact
//! downloads. A failed download starts over.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use libtea_core::checksum::ContentDecoder;
use libtea_core::http_cache::{CacheEntryMeta, NotStored};
use libtea_core::{
    escape_control_chars, Artifact, ChecksumVerifier, IntegrityOutcome, IntegrityReport,
};
use reqwest::header::{CONTENT_ENCODING, CONTENT_TYPE, ETAG};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use url::Url;

use crate::cache::{now_secs, Cache};
use crate::error::ClientError;
use crate::transport::{
    decide_storage, display_url, header_str, map_api_error, storable_headers, Call, StreamStart,
};
use crate::Service;

/// What a streamed download reports. The bytes went to the sink.
///
/// *Experimental.*
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StreamedDownload {
    /// Number of decoded bytes written to the sink.
    pub bytes_written: u64,
    /// Response `Content-Type`.
    pub content_type: Option<String>,
    /// HTTP content coding that was removed, when one was declared.
    pub content_encoding: Option<String>,
    /// `ETag` as received. This is not a checksum.
    pub etag: Option<String>,
    /// Whether the tag is strong. A weak tag is not byte-identity proof.
    pub strong_etag: bool,
    /// Final URL after redirects.
    pub final_url: String,
    /// Checksum report over the decoded bytes. [`IntegrityOutcome::Verified`]
    /// is integrity of those bytes, not publisher authenticity.
    /// [`IntegrityOutcome::NotDeclared`] means nothing was checked.
    pub integrity: IntegrityReport,
}

/// A streamed download that was committed to a file.
///
/// *Experimental.*
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileDownload {
    /// The destination path, as given.
    pub path: PathBuf,
    /// Details of the download.
    pub download: StreamedDownload,
}

/// Options for the file-writing download methods.
///
/// *Experimental.*
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileDownloadOptions {
    /// Replace an existing regular file at the destination. Default `false`:
    /// an existing path is refused. A symbolic link at the destination is
    /// refused either way.
    pub overwrite: bool,
}

impl FileDownloadOptions {
    /// Default options: refuse an existing destination.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set [`Self::overwrite`].
    #[must_use]
    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }
}

fn io_error(context: &str, err: &std::io::Error) -> ClientError {
    ClientError::Io(escape_control_chars(&format!("{context}: {}", err.kind())))
}

impl Service {
    /// Stream one format of an artifact revision into `writer`.
    ///
    /// *Experimental.* The writer receives bytes **before** the checksum
    /// comparison is possible. Use it only if you can discard them on error;
    /// prefer [`Service::download_format_to_file`]. The writer is flushed on
    /// success.
    ///
    /// Like [`Service::download_format`], an external `url` is fetched without
    /// the TEA access token.
    ///
    /// # Errors
    ///
    /// As [`Service::download_format`], plus [`ClientError::Io`] when the
    /// writer fails. [`ClientError::Integrity`] is returned after the bytes
    /// were written.
    pub async fn download_format_to_writer<W>(
        &self,
        artifact: &Artifact,
        media_type: &str,
        writer: &mut W,
    ) -> Result<StreamedDownload, ClientError>
    where
        W: AsyncWrite + Unpin + Send,
    {
        let (url, api_prefix, checksums) = self.format_source(artifact, media_type)?;
        self.stream_to_writer(url, api_prefix, &checksums, writer)
            .await
    }

    /// Stream one format of an artifact revision into a file.
    ///
    /// *Experimental.* The file at `dest` appears only after the stream ended
    /// cleanly and the checksums passed the client's policy; see the module
    /// documentation. `dest`'s parent directory must exist.
    ///
    /// A `NotDeclared` integrity outcome is committed, as in
    /// [`Service::download_format`]; inspect
    /// [`StreamedDownload::integrity`] when that matters.
    ///
    /// # Errors
    ///
    /// As [`Service::download_format_to_writer`]; also [`ClientError::Io`]
    /// when `dest` exists and `overwrite` is not set, is a symbolic link, or
    /// cannot be created.
    pub async fn download_format_to_file(
        &self,
        artifact: &Artifact,
        media_type: &str,
        dest: &Path,
        options: &FileDownloadOptions,
    ) -> Result<FileDownload, ClientError> {
        let (url, api_prefix, checksums) = self.format_source(artifact, media_type)?;
        self.stream_to_file(url, api_prefix, &checksums, dest, options)
            .await
    }

    /// Stream `GET /artifact/{uuid}/{version}/download` into a file.
    ///
    /// *Experimental.* `checksums` are the ones declared for the selected
    /// format; with none, the outcome is `NotDeclared`.
    ///
    /// # Errors
    ///
    /// As [`Service::download_format_to_file`].
    pub async fn download_artifact_to_file(
        &self,
        id: libtea_core::ArtifactId,
        version: u64,
        media_type: Option<&str>,
        checksums: &[libtea_core::Checksum],
        dest: &Path,
        options: &FileDownloadOptions,
    ) -> Result<FileDownload, ClientError> {
        let (url, api_prefix) =
            self.api_source(&format!("artifact/{id}/{version}/download"), media_type)?;
        self.stream_to_file(url, api_prefix, checksums, dest, options)
            .await
    }

    /// Stream `GET /artifact/{uuid}/latest/download` into a file.
    ///
    /// *Experimental.* See [`Service::download_artifact_to_file`].
    ///
    /// # Errors
    ///
    /// As [`Service::download_format_to_file`].
    pub async fn download_latest_artifact_to_file(
        &self,
        id: libtea_core::ArtifactId,
        media_type: Option<&str>,
        checksums: &[libtea_core::Checksum],
        dest: &Path,
        options: &FileDownloadOptions,
    ) -> Result<FileDownload, ClientError> {
        let (url, api_prefix) =
            self.api_source(&format!("artifact/{id}/latest/download"), media_type)?;
        self.stream_to_file(url, api_prefix, checksums, dest, options)
            .await
    }

    fn format_source(
        &self,
        artifact: &Artifact,
        media_type: &str,
    ) -> Result<(Url, Option<Url>, Vec<libtea_core::Checksum>), ClientError> {
        let format = artifact.format(media_type).ok_or_else(|| {
            ClientError::Model(libtea_core::ModelError::Invalid(format!(
                "artifact has no format `{media_type}`"
            )))
        })?;
        format.validate()?;
        let checksums = format.checksums.clone().unwrap_or_default();
        if let Some(external) = &format.url {
            let url = Url::parse(external).map_err(|_| {
                ClientError::Model(libtea_core::ModelError::Invalid(
                    "external artifact URL".into(),
                ))
            })?;
            return Ok((url, None, checksums));
        }
        let (url, prefix) = self.api_source(
            &format!("artifact/{}/{}/download", artifact.uuid, artifact.version),
            Some(media_type),
        )?;
        Ok((url, prefix, checksums))
    }

    fn api_source(
        &self,
        path: &str,
        media_type: Option<&str>,
    ) -> Result<(Url, Option<Url>), ClientError> {
        let mut query = Vec::new();
        if let Some(media_type) = media_type {
            query.push(("mediaType", media_type));
        }
        let url = crate::transport::operation_url(self.prefix_ref(), path, &query)?;
        Ok((url, Some(self.prefix_ref().clone())))
    }

    async fn stream_to_file(
        &self,
        url: Url,
        api_prefix: Option<Url>,
        checksums: &[libtea_core::Checksum],
        dest: &Path,
        options: &FileDownloadOptions,
    ) -> Result<FileDownload, ClientError> {
        let name = dest.file_name().ok_or_else(|| {
            ClientError::Io(escape_control_chars(&format!(
                "{} has no file name",
                dest.display()
            )))
        })?;
        check_destination(dest, options)?;
        let dir = dest
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));

        let (temp_path, file) = create_temp(dir, name).await?;
        // Declared before the writer, so it is dropped (and removes the file)
        // after the writer closed it.
        let mut guard = TempFile {
            path: Some(temp_path.clone()),
        };
        let mut writer = tokio::io::BufWriter::new(file);
        let download = self
            .stream_to_writer(url, api_prefix, checksums, &mut writer)
            .await?;
        writer
            .get_ref()
            .sync_all()
            .await
            .map_err(|err| io_error("syncing the download", &err))?;
        drop(writer);

        // The destination may have changed while the download ran.
        check_destination(dest, options)?;
        commit_file(&temp_path, dest, options)?;
        guard.path = None;
        Ok(FileDownload {
            path: dest.to_path_buf(),
            download,
        })
    }

    async fn stream_to_writer<W>(
        &self,
        url: Url,
        api_prefix: Option<Url>,
        checksums: &[libtea_core::Checksum],
        writer: &mut W,
    ) -> Result<StreamedDownload, ClientError>
    where
        W: AsyncWrite + Unpin + Send,
    {
        let inner = &self.client_ref().inner;
        let max = inner.limits.max_artifact_bytes;
        let mut call = Call {
            method: reqwest::Method::GET,
            url,
            api_prefix,
            artifact: true,
            body: None,
            content_type: None,
            basic_auth: false,
            if_none_match: None,
            if_modified_since: None,
            max_body: max,
        };

        // Cache lookup. A streamed body is stored already decoded, so an entry
        // that says otherwise is not used.
        let cache = inner.cache.clone();
        let key = cache.as_ref().and_then(|_| inner.cache_key_for(&call.url));
        let mut existing = None;
        if let Some(cache) = &cache {
            let Some(key) = &key else {
                return if cache.offline {
                    Err(ClientError::Offline)
                } else {
                    self.stream_network(call, checksums, writer, None, None)
                        .await
                };
            };
            // A cache hit is subject to the destination rules and the size limit
            // of this client, whoever stored it and whenever.
            inner.policy.check_url(&call.url)?;
            existing = cache.load(key).await.filter(|(meta, _)| meta.decoded);
            if existing.as_ref().is_some_and(|(_, body)| body.len() > max) {
                cache.remove(key).await;
                if cache.offline {
                    return Err(ClientError::TooLarge);
                }
                existing = None;
            }
            if cache.offline {
                let (meta, body) = existing.ok_or(ClientError::Offline)?;
                return serve_cached(
                    inner,
                    Some((cache, key)),
                    &call.url,
                    &meta,
                    &body,
                    checksums,
                    writer,
                )
                .await;
            }
            if let Some((meta, _)) = &existing {
                call.if_none_match = meta.header("etag").map(str::to_owned);
                call.if_modified_since = meta.header("last-modified").map(str::to_owned);
            }
        }
        self.stream_network(call, checksums, writer, key, existing)
            .await
    }

    async fn stream_network<W>(
        &self,
        call: Call,
        checksums: &[libtea_core::Checksum],
        writer: &mut W,
        key: Option<String>,
        existing: Option<(CacheEntryMeta, Vec<u8>)>,
    ) -> Result<StreamedDownload, ClientError>
    where
        W: AsyncWrite + Unpin + Send,
    {
        let inner = &self.client_ref().inner;
        let max = inner.limits.max_artifact_bytes;
        let cache = inner.cache.clone();
        let url = call.url.clone();
        let start = inner.send_streaming(call).await?;
        let mut body = match start {
            StreamStart::Body(body) => body,
            StreamStart::Other(raw) => {
                if let (Some(cache), Some(key)) = (&cache, &key) {
                    if raw.status.as_u16() == 304 {
                        if let Some((meta, data)) = existing {
                            let revalidated =
                                inner.revalidated_meta(cache, meta.clone(), &raw.headers, false);
                            let shown = revalidated.as_ref().unwrap_or(&meta);
                            let served = serve_cached(
                                inner,
                                Some((cache, key)),
                                &url,
                                shown,
                                &data,
                                checksums,
                                writer,
                            )
                            .await;
                            // The entry's age restarts only after its bytes passed
                            // the checksum check; a failure has already evicted it.
                            if served.is_ok() {
                                match revalidated {
                                    Some(meta) => cache.store(key, meta, data).await,
                                    None => cache.remove(key).await,
                                }
                            }
                            return served;
                        }
                    } else if matches!(raw.status.as_u16(), 404 | 410) {
                        cache.remove(key).await;
                    }
                }
                return Err(map_api_error(&raw));
            }
        };

        // Decide whether to keep a copy while the body streams past.
        let mut tee = None;
        if let (Some(cache), Some(key)) = (&cache, &key) {
            if body.status.as_u16() == 200 {
                match decide_storage(
                    &body.headers,
                    body.authenticated,
                    cache.config.authenticated,
                ) {
                    Ok(()) => tee = cache.start_tee(key).await,
                    Err(NotStored::NoStore) => cache.remove(key).await,
                    Err(_) => {}
                }
            }
        }

        let encoding = header_str(&body.headers, CONTENT_ENCODING).map(str::to_owned);
        let mut decoder = ContentDecoder::new(encoding.as_deref(), max)?;
        let mut verifier = ChecksumVerifier::with_policy(checksums, inner.checksum_policy);
        let mut written = 0u64;
        while let Some(chunk) = body.chunk().await? {
            decoder.push(&chunk)?;
            let out = decoder.take_output();
            tee_write(&mut tee, &out).await;
            sink(&mut verifier, writer, &mut written, &out).await?;
        }
        // `finish` is what rejects a truncated coding stream.
        let rest = decoder.finish()?;
        tee_write(&mut tee, &rest).await;
        sink(&mut verifier, writer, &mut written, &rest).await?;
        writer
            .flush()
            .await
            .map_err(|err| io_error("flushing the download", &err))?;
        let integrity = checked(verifier)?;
        // Only a body that passed its checks reaches the cache.
        if let (Some(tee), Some(key)) = (tee.take(), &key) {
            let meta = CacheEntryMeta::new(
                key,
                display_url(&url),
                200,
                now_secs(),
                true,
                &storable_headers(&body.headers),
            );
            tee.finish(meta).await;
        }
        let etag = header_str(&body.headers, ETAG).map(str::to_owned);
        let strong_etag = etag
            .as_ref()
            .is_some_and(|value| !value.trim().starts_with("W/"));
        Ok(StreamedDownload {
            bytes_written: written,
            content_type: header_str(&body.headers, CONTENT_TYPE).map(str::to_owned),
            content_encoding: encoding,
            etag,
            strong_etag,
            final_url: body.final_url.to_string(),
            integrity,
        })
    }
}

/// Add decoded bytes to the cache copy; drop the copy if it cannot continue.
async fn tee_write(tee: &mut Option<crate::cache::CacheTee>, bytes: &[u8]) {
    if let Some(active) = tee.as_mut() {
        if !active.write(bytes).await {
            *tee = None;
        }
    }
}

/// The integrity report, or the error when the outcome is not acceptable.
fn checked(verifier: ChecksumVerifier) -> Result<IntegrityReport, ClientError> {
    let integrity = verifier.finish();
    if matches!(
        integrity.outcome,
        IntegrityOutcome::Verified | IntegrityOutcome::NotDeclared
    ) {
        Ok(integrity)
    } else {
        Err(ClientError::Integrity {
            outcome: integrity.outcome.clone(),
            report: integrity,
        })
    }
}

/// Answer from a cache entry. The bytes go through the same checksum check as
/// a download: the cache is not trusted.
async fn serve_cached<W>(
    inner: &crate::transport::ClientInner,
    origin: Option<(&Arc<Cache>, &str)>,
    url: &Url,
    meta: &CacheEntryMeta,
    body: &[u8],
    checksums: &[libtea_core::Checksum],
    writer: &mut W,
) -> Result<StreamedDownload, ClientError>
where
    W: AsyncWrite + Unpin + Send,
{
    let mut verifier = ChecksumVerifier::with_policy(checksums, inner.checksum_policy);
    let mut written = 0u64;
    sink(&mut verifier, writer, &mut written, body).await?;
    writer
        .flush()
        .await
        .map_err(|err| io_error("flushing the download", &err))?;
    let integrity = match checked(verifier) {
        Ok(integrity) => integrity,
        Err(error) => {
            // Bytes from the cache that fail their checksums are not kept.
            if let Some((cache, key)) = origin {
                cache.remove(key).await;
            }
            return Err(error);
        }
    };
    let etag = meta.header("etag").map(str::to_owned);
    let strong_etag = etag
        .as_ref()
        .is_some_and(|value| !value.trim().starts_with("W/"));
    Ok(StreamedDownload {
        bytes_written: written,
        content_type: meta.header("content-type").map(str::to_owned),
        content_encoding: meta.header("content-encoding").map(str::to_owned),
        etag,
        strong_etag,
        final_url: url.to_string(),
        integrity,
    })
}

async fn sink<W>(
    verifier: &mut ChecksumVerifier,
    writer: &mut W,
    written: &mut u64,
    bytes: &[u8],
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin + Send,
{
    if bytes.is_empty() {
        return Ok(());
    }
    verifier.update(bytes);
    writer
        .write_all(bytes)
        .await
        .map_err(|err| io_error("writing the download", &err))?;
    *written = written.saturating_add(bytes.len() as u64);
    Ok(())
}

/// Removes the temporary file unless it was committed. Runs on drop, so a
/// cancelled future cleans up too (best effort; a failed removal is ignored).
struct TempFile {
    path: Option<PathBuf>,
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Refuse a destination that must not be written, before and after the download.
fn check_destination(dest: &Path, options: &FileDownloadOptions) -> Result<(), ClientError> {
    match std::fs::symlink_metadata(dest) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(ClientError::Io(escape_control_chars(&format!(
                "refusing to write {}: it is a symbolic link",
                dest.display()
            ))))
        }
        Ok(_) if !options.overwrite => Err(ClientError::Io(escape_control_chars(&format!(
            "{} already exists; overwrite was not requested",
            dest.display()
        )))),
        Ok(meta) if !meta.is_file() => Err(ClientError::Io(escape_control_chars(&format!(
            "{} exists and is not a regular file",
            dest.display()
        )))),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(io_error("inspecting the destination", &err)),
    }
}

async fn create_temp(
    dir: &Path,
    name: &std::ffi::OsStr,
) -> Result<(PathBuf, tokio::fs::File), ClientError> {
    let mut last = None;
    for _ in 0..16 {
        let mut temp_name = std::ffi::OsString::from(".");
        temp_name.push(name);
        temp_name.push(format!(
            ".libtea-{}-{}.part",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(temp_name);
        // `create_new` never follows a symbolic link and fails if the name exists.
        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => last = Some(err),
            Err(err) => return Err(io_error("creating the temporary file", &err)),
        }
    }
    Err(match last {
        Some(err) => io_error("creating the temporary file", &err),
        None => ClientError::Io("creating the temporary file".into()),
    })
}

/// Move the finished temporary file to `dest`.
///
/// Without `overwrite` the move is a hard link, which fails if `dest` appeared
/// in the meantime. Where hard links are unavailable it falls back to a rename
/// after the existence check, which leaves a small window.
fn commit_file(temp: &Path, dest: &Path, options: &FileDownloadOptions) -> Result<(), ClientError> {
    if !options.overwrite {
        match std::fs::hard_link(temp, dest) {
            Ok(()) => {
                let _ = std::fs::remove_file(temp);
                return Ok(());
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(io_error("committing the download", &err));
            }
            Err(_) => {}
        }
    }
    std::fs::rename(temp, dest).map_err(|err| io_error("committing the download", &err))
}
