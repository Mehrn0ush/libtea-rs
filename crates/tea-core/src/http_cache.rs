//! Pure pieces of the client's HTTP cache: key derivation, `Cache-Control` and
//! `Retry-After` parsing, the storage decision, and the on-disk entry format.
//!
//! *Experimental until 0.9.* This module does no I/O and reads no clock; callers
//! pass times as Unix seconds. `libtea-client` does the file work
//! (`docs/cache.md`).
//!
//! **A cache entry is not a trusted statement.** The entry format carries a
//! length and a SHA-256 of the body so that truncation and accidental
//! corruption are detected. Anyone who can write the cache directory can write
//! a consistent entry, so the hash is not authentication. Artifact bytes are
//! therefore checked against the declared checksums every time they are used,
//! and the directory is created private (`docs/cache.md`).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Entry layout version written and read by this crate.
pub const CACHE_FORMAT_VERSION: u32 = 1;

const MAGIC: &[u8] = b"LIBTEA-CACHE 1\n";
/// The entry ends with `<meta JSON>\n<10 digits>\n`; this is the length of the
/// last part.
const LEN_FIELD: usize = 11;
/// Largest metadata block accepted.
pub const MAX_META_BYTES: usize = 64 * 1024;
const MAX_HEADERS: usize = 16;
const MAX_HEADER_VALUE: usize = 8 * 1024;

/// Response headers an entry keeps, and nothing else.
pub const STORED_HEADERS: &[&str] = &[
    "content-type",
    "content-encoding",
    "etag",
    "last-modified",
    "cache-control",
    "content-location",
    "vary",
    "repr-digest",
];

/// Whether a header may be written to an entry: allow-listed, and a value of
/// visible ASCII (and tab) no longer than 8 KiB.
#[must_use]
pub fn is_storable_header(name: &str, value: &str) -> bool {
    STORED_HEADERS.contains(&name)
        && value.len() <= MAX_HEADER_VALUE
        && value
            .bytes()
            .all(|b| b == b'\t' || (0x20..=0x7e).contains(&b))
}

/// The directives of `Cache-Control` this client acts on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CacheControl {
    /// `no-store`: nothing may be stored.
    pub no_store: bool,
    /// `private`: meant for a single user's cache.
    pub private: bool,
    /// `no-cache`: always revalidate before use.
    pub no_cache: bool,
    /// `max-age` in seconds; saturates instead of overflowing.
    pub max_age: Option<u64>,
}

impl CacheControl {
    /// Parse every `Cache-Control` header line of a response. Unknown
    /// directives and malformed values are ignored; the strictest reading of
    /// repeated directives wins.
    #[must_use]
    pub fn parse(lines: &[&str]) -> Self {
        let mut out = Self::default();
        for line in lines {
            for part in line.split(',') {
                let part = part.trim();
                let (name, value) = match part.split_once('=') {
                    Some((n, v)) => (n.trim(), Some(v.trim().trim_matches('"'))),
                    None => (part, None),
                };
                match name.to_ascii_lowercase().as_str() {
                    "no-store" => out.no_store = true,
                    "private" => out.private = true,
                    "no-cache" => out.no_cache = true,
                    "max-age" => {
                        if let Some(value) = value {
                            if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
                                let seconds = value.parse::<u64>().unwrap_or(u64::MAX);
                                out.max_age = Some(match out.max_age {
                                    Some(existing) => existing.min(seconds),
                                    None => seconds,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }
}

/// Why a response was not stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NotStored {
    /// Only `200` responses are stored.
    Status,
    /// `Cache-Control: no-store`.
    NoStore,
    /// `Cache-Control: private` and shared-credential caching was not enabled.
    Private,
    /// The request carried credentials and authenticated caching was not enabled.
    Authenticated,
    /// `Vary: *`.
    VaryStar,
}

/// Decide whether a response may be stored.
///
/// A private per-user cache may in principle keep `private` responses; this
/// client keeps them only when the caller opted in to authenticated caching,
/// the conservative reading of the F6 requirement.
///
/// # Errors
///
/// Returns the reason when the response must not be stored.
pub fn storage_decision(
    status: u16,
    control: &CacheControl,
    vary: Option<&str>,
    request_was_authenticated: bool,
    allow_authenticated: bool,
) -> Result<(), NotStored> {
    if status != 200 {
        return Err(NotStored::Status);
    }
    if control.no_store {
        return Err(NotStored::NoStore);
    }
    if vary.is_some_and(|v| v.split(',').any(|t| t.trim() == "*")) {
        return Err(NotStored::VaryStar);
    }
    if request_was_authenticated && !allow_authenticated {
        return Err(NotStored::Authenticated);
    }
    if control.private && !allow_authenticated {
        return Err(NotStored::Private);
    }
    Ok(())
}

/// Seconds to wait according to a `Retry-After` value.
///
/// Accepts delta-seconds (digits only) or an IMF-fixdate such as
/// `Sun, 06 Nov 1994 08:49:37 GMT`. A date in the past gives `Some(0)`.
/// Anything else, including the obsolete date formats, gives `None`.
#[must_use]
pub fn parse_retry_after(value: &str, now_epoch_secs: u64) -> Option<u64> {
    let value = value.trim();
    if !value.is_empty() && value.len() <= 20 && value.bytes().all(|b| b.is_ascii_digit()) {
        return Some(value.parse::<u64>().unwrap_or(u64::MAX));
    }
    let when = parse_imf_fixdate(value)?;
    Some(when.saturating_sub(now_epoch_secs))
}

fn parse_imf_fixdate(value: &str) -> Option<u64> {
    // "Sun, 06 Nov 1994 08:49:37 GMT"
    let b = value.as_bytes();
    // ASCII only, so every byte offset below is a char boundary for `&value[..]`.
    if b.len() != 29 || !value.is_ascii() || &b[3..5] != b", " || &b[25..] != b" GMT" {
        return None;
    }
    let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    if !day_names.contains(&&value[..3]) {
        return None;
    }
    let digits = |range: std::ops::Range<usize>| -> Option<u64> {
        let part = &value[range];
        if part.bytes().all(|c| c.is_ascii_digit()) {
            part.parse().ok()
        } else {
            None
        }
    };
    let day = digits(5..7)?;
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|m| *m == &value[8..11])? as u64
        + 1;
    let year = digits(12..16)?;
    if b[7] != b' ' || b[11] != b' ' || b[16] != b' ' || b[19] != b':' || b[22] != b':' {
        return None;
    }
    let (hour, minute, second) = (digits(17..19)?, digits(20..22)?, digits(23..25)?);
    if !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 || year < 1970 {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day > month_days[(month - 1) as usize] {
        return None;
    }
    let mut days = 0u64;
    for y in 1970..year {
        days += if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
    }
    days += month_days[..(month - 1) as usize].iter().sum::<u64>() + (day - 1);
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second.min(59))
}

/// The cache key of a request: lowercase hex SHA-256 over the method, the
/// normalized URL, and a digest of the origin and the API key id (never the
/// secret). Requests from different keys, or to different origins, never share
/// an entry.
///
/// `url` should already be normalized: lowercase scheme and host, default port
/// removed, no fragment, no userinfo. The query is part of the key verbatim.
#[must_use]
pub fn cache_key(method: &str, url: &str, origin: &str, key_id: Option<&str>) -> String {
    let mut scope = Sha256::new();
    scope.update(b"libtea-cache-scope-v1\0");
    scope.update(origin.as_bytes());
    scope.update(b"\0");
    scope.update(key_id.unwrap_or("").as_bytes());
    scope.update(b"\0");
    scope.update([u8::from(key_id.is_some())]);
    let mut key = Sha256::new();
    key.update(b"libtea-cache-key-v1\0");
    key.update(method.as_bytes());
    key.update(b"\0");
    key.update(url.as_bytes());
    key.update(b"\0");
    key.update(scope.finalize());
    hex(&key.finalize())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// Whether `key` looks like a key produced by [`cache_key`].
#[must_use]
pub fn is_cache_key(key: &str) -> bool {
    key.len() == 64 && key.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Metadata of one stored response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[non_exhaustive]
pub struct CacheEntryMeta {
    /// Layout version, [`CACHE_FORMAT_VERSION`].
    pub version: u32,
    /// The entry's cache key; must equal the key it is looked up under.
    pub key: String,
    /// The request URL without query or fragment. For display only.
    pub request_url: String,
    /// HTTP status of the stored response.
    pub status: u16,
    /// When the body was first stored (Unix seconds).
    pub stored_at: u64,
    /// When the body was last confirmed current (Unix seconds).
    pub validated_at: u64,
    /// Whether the body is already free of HTTP content coding.
    pub decoded: bool,
    /// Allow-listed response headers.
    pub headers: Vec<(String, String)>,
    /// Body length in bytes (set by the encoder).
    pub body_len: u64,
    /// Lowercase hex SHA-256 of the body (set by the encoder).
    pub body_sha256: String,
}

impl CacheEntryMeta {
    /// Start metadata for a response. Headers that are not storable are dropped.
    #[must_use]
    pub fn new(
        key: impl Into<String>,
        request_url: impl Into<String>,
        status: u16,
        now: u64,
        decoded: bool,
        headers: &[(String, String)],
    ) -> Self {
        let headers = headers
            .iter()
            .filter(|(n, v)| is_storable_header(n, v))
            .take(MAX_HEADERS)
            .cloned()
            .collect();
        Self {
            version: CACHE_FORMAT_VERSION,
            key: key.into(),
            request_url: request_url.into(),
            status,
            stored_at: now,
            validated_at: now,
            decoded,
            headers,
            body_len: 0,
            body_sha256: String::new(),
        }
    }

    /// A stored header value, by lowercase name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Why an entry was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EntryError {
    /// Not an entry of this version.
    Format,
    /// The metadata is malformed or has an out-of-range field.
    Meta,
    /// The entry belongs to a different key.
    KeyMismatch,
    /// The body is shorter, longer or different from what the metadata states.
    Body,
    /// The body is larger than the caller's limit.
    TooLarge,
}

impl std::fmt::Display for EntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Format => "not a cache entry of this version",
            Self::Meta => "cache entry metadata is invalid",
            Self::KeyMismatch => "cache entry belongs to another key",
            Self::Body => "cache entry body does not match its metadata",
            Self::TooLarge => "cache entry is larger than the limit",
        })
    }
}

impl std::error::Error for EntryError {}

/// Builds an entry while the body is still arriving.
///
/// Write [`EntryBuilder::prefix`], then each body chunk (also passed to
/// [`EntryBuilder::update`]), then [`EntryBuilder::trailer`].
pub struct EntryBuilder {
    hasher: Sha256,
    len: u64,
}

impl Default for EntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EntryBuilder {
    /// Start an entry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
            len: 0,
        }
    }

    /// The bytes that start every entry.
    #[must_use]
    pub fn prefix() -> &'static [u8] {
        MAGIC
    }

    /// Account for one body chunk.
    pub fn update(&mut self, chunk: &[u8]) {
        self.hasher.update(chunk);
        self.len = self.len.saturating_add(chunk.len() as u64);
    }

    /// Body bytes seen so far.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Whether no body byte was seen.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The bytes that end the entry: the metadata (with length and digest
    /// filled in) and its length.
    ///
    /// # Errors
    ///
    /// Returns an error when the metadata cannot be serialised or is too large.
    pub fn trailer(self, mut meta: CacheEntryMeta) -> Result<Vec<u8>, EntryError> {
        meta.body_len = self.len;
        meta.body_sha256 = hex(&self.hasher.finalize());
        let mut json = serde_json::to_vec(&meta).map_err(|_| EntryError::Meta)?;
        if json.len() > MAX_META_BYTES {
            return Err(EntryError::Meta);
        }
        let length = json.len();
        json.push(b'\n');
        json.extend_from_slice(format!("{length:010}\n").as_bytes());
        Ok(json)
    }
}

/// Encode a whole entry in memory.
///
/// # Errors
///
/// Returns an error when the metadata cannot be serialised or is too large.
pub fn encode_entry(meta: CacheEntryMeta, body: &[u8]) -> Result<Vec<u8>, EntryError> {
    let mut builder = EntryBuilder::new();
    builder.update(body);
    let trailer = builder.trailer(meta)?;
    let mut out = Vec::with_capacity(MAGIC.len() + body.len() + trailer.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(body);
    out.extend_from_slice(&trailer);
    Ok(out)
}

/// Decode and check an entry.
///
/// The key must equal `expect_key`, the body must have exactly the stated
/// length and SHA-256, and no field may be out of range. Nothing in the entry
/// is believed beyond that: see the module documentation.
///
/// # Errors
///
/// Returns why the entry was refused.
pub fn decode_entry(
    bytes: &[u8],
    expect_key: &str,
    max_body: usize,
) -> Result<(CacheEntryMeta, Vec<u8>), EntryError> {
    if !bytes.starts_with(MAGIC) || bytes.len() < MAGIC.len() + LEN_FIELD + 2 {
        return Err(EntryError::Format);
    }
    let end = bytes.len();
    let len_field = &bytes[end - LEN_FIELD..];
    if len_field[10] != b'\n' || !len_field[..10].iter().all(u8::is_ascii_digit) {
        return Err(EntryError::Format);
    }
    let meta_len: usize = std::str::from_utf8(&len_field[..10])
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or(EntryError::Format)?;
    if meta_len == 0 || meta_len > MAX_META_BYTES || meta_len + 1 + LEN_FIELD + MAGIC.len() > end {
        return Err(EntryError::Meta);
    }
    let meta_end = end - LEN_FIELD - 1;
    if bytes[meta_end] != b'\n' {
        return Err(EntryError::Format);
    }
    let meta_start = meta_end - meta_len;
    let meta: CacheEntryMeta =
        serde_json::from_slice(&bytes[meta_start..meta_end]).map_err(|_| EntryError::Meta)?;
    check_meta(&meta)?;
    if meta.key != expect_key {
        return Err(EntryError::KeyMismatch);
    }
    let body = &bytes[MAGIC.len()..meta_start];
    if usize::try_from(meta.body_len) != Ok(body.len()) {
        return Err(EntryError::Body);
    }
    if body.len() > max_body {
        return Err(EntryError::TooLarge);
    }
    if hex(&Sha256::digest(body)) != meta.body_sha256 {
        return Err(EntryError::Body);
    }
    Ok((meta, body.to_vec()))
}

fn check_meta(meta: &CacheEntryMeta) -> Result<(), EntryError> {
    let ok = meta.version == CACHE_FORMAT_VERSION
        && is_cache_key(&meta.key)
        && (100..=599).contains(&meta.status)
        && meta.validated_at >= meta.stored_at
        && meta.request_url.len() <= 2048
        && meta.headers.len() <= MAX_HEADERS
        && meta.headers.iter().all(|(n, v)| is_storable_header(n, v))
        && meta.body_sha256.len() == 64
        && meta
            .body_sha256
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if ok {
        Ok(())
    } else {
        Err(EntryError::Meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://tea.example.com/v1.0.0/product/1";

    fn key() -> String {
        cache_key("GET", URL, "https://tea.example.com", None)
    }

    fn meta() -> CacheEntryMeta {
        CacheEntryMeta::new(
            key(),
            "https://tea.example.com/v1.0.0/product/1",
            200,
            1_000,
            false,
            &[
                ("etag".into(), "\"abc\"".into()),
                ("content-type".into(), "application/json".into()),
                ("set-cookie".into(), "dropped=1".into()),
                ("x-secret".into(), "dropped".into()),
            ],
        )
    }

    #[test]
    fn keys_depend_on_method_url_origin_and_key_id_and_nothing_else() {
        let base = cache_key("GET", URL, "https://a", Some("id-1"));
        assert!(is_cache_key(&base));
        assert_eq!(base, cache_key("GET", URL, "https://a", Some("id-1")));
        for other in [
            cache_key("HEAD", URL, "https://a", Some("id-1")),
            cache_key(
                "GET",
                "https://tea.example.com/v1.0.0/product/2",
                "https://a",
                Some("id-1"),
            ),
            cache_key("GET", URL, "https://b", Some("id-1")),
            cache_key("GET", URL, "https://a", Some("id-2")),
            cache_key("GET", URL, "https://a", None),
            cache_key("GET", URL, "https://a", Some("")),
        ] {
            assert_ne!(base, other);
        }
    }

    #[test]
    fn the_key_never_contains_the_key_id() {
        let key = cache_key("GET", URL, "https://a", Some("my-visible-key-id"));
        assert!(!key.contains("my-visible-key-id"));
    }

    #[test]
    fn entries_round_trip_and_drop_unlisted_headers() {
        let bytes = encode_entry(meta(), b"{\"a\":1}").unwrap();
        let (decoded, body) = decode_entry(&bytes, &key(), 1 << 20).unwrap();
        assert_eq!(body, b"{\"a\":1}");
        assert_eq!(decoded.status, 200);
        assert_eq!(decoded.header("etag"), Some("\"abc\""));
        assert_eq!(decoded.header("set-cookie"), None);
        assert_eq!(decoded.header("x-secret"), None);
        assert_eq!(decoded.body_len, 7);
    }

    #[test]
    fn streamed_and_in_memory_entries_are_identical() {
        let whole = encode_entry(meta(), b"hello world").unwrap();
        let mut builder = EntryBuilder::new();
        builder.update(b"hello ");
        builder.update(b"world");
        let mut streamed = EntryBuilder::prefix().to_vec();
        streamed.extend_from_slice(b"hello world");
        streamed.extend(builder.trailer(meta()).unwrap());
        assert_eq!(whole, streamed);
    }

    #[test]
    fn every_kind_of_damage_is_refused() {
        let good = encode_entry(meta(), b"payload-bytes").unwrap();
        // truncation at every length
        for cut in 0..good.len() {
            assert!(
                decode_entry(&good[..cut], &key(), 1 << 20).is_err(),
                "cut {cut}"
            );
        }
        // A flipped bit in the magic or the body is caught. (The timestamps in
        // the metadata are not covered by the body digest and are not trusted.)
        let body_at = EntryBuilder::prefix().len();
        for at in (0..body_at).chain(body_at..body_at + 13) {
            let mut bad = good.clone();
            bad[at] ^= 0x01;
            assert!(decode_entry(&bad, &key(), 1 << 20).is_err(), "flip at {at}");
        }
        // another key
        let other = cache_key("GET", "https://x/y", "https://x", None);
        assert_eq!(
            decode_entry(&good, &other, 1 << 20).unwrap_err(),
            EntryError::KeyMismatch
        );
        // too large
        assert_eq!(
            decode_entry(&good, &key(), 3).unwrap_err(),
            EntryError::TooLarge
        );
        // garbage
        for junk in [
            &b""[..],
            b"LIBTEA-CACHE 1\n",
            b"\x00\x01\x02",
            &[b'a'; 4096],
        ] {
            assert!(decode_entry(junk, &key(), 1 << 20).is_err());
        }
    }

    #[test]
    fn hostile_metadata_is_refused() {
        let body = b"x";
        let mut bad = meta();
        bad.status = 99;
        assert!(decode_entry(&encode_entry(bad, body).unwrap(), &key(), 10).is_err());
        let mut bad = meta();
        bad.validated_at = 1;
        assert!(decode_entry(&encode_entry(bad, body).unwrap(), &key(), 10).is_err());
        let mut bad = meta();
        bad.headers.push(("set-cookie".into(), "a=b".into()));
        assert!(decode_entry(&encode_entry(bad, body).unwrap(), &key(), 10).is_err());
        let mut bad = meta();
        bad.headers = vec![("etag".into(), "a\u{1b}[31m".into())];
        assert!(decode_entry(&encode_entry(bad, body).unwrap(), &key(), 10).is_err());
        let mut bad = meta();
        bad.request_url = "x".repeat(5000);
        assert!(decode_entry(&encode_entry(bad, body).unwrap(), &key(), 10).is_err());
        let mut bad = meta();
        bad.headers = vec![("etag".into(), "x".repeat(9000))];
        // an oversized value is dropped when the metadata is built...
        assert!(CacheEntryMeta::new(key(), "u", 200, 1, false, &bad.headers)
            .headers
            .is_empty());
    }

    #[test]
    fn cache_control_is_parsed_defensively() {
        let p = |lines: &[&str]| CacheControl::parse(lines);
        assert!(p(&["no-store"]).no_store);
        assert!(p(&["max-age=5, No-Store"]).no_store);
        assert!(p(&["public", "private"]).private);
        assert!(p(&["NO-CACHE"]).no_cache);
        assert_eq!(p(&["max-age=60"]).max_age, Some(60));
        assert_eq!(p(&["max-age=\"60\""]).max_age, Some(60));
        assert_eq!(p(&["max-age=60", "max-age=10"]).max_age, Some(10));
        assert_eq!(
            p(&["max-age=99999999999999999999999999"]).max_age,
            Some(u64::MAX)
        );
        for junk in [
            "max-age=-1",
            "max-age=",
            "max-age=1e3",
            "max-age= ",
            "=",
            ",,,",
            "max-age=1 2",
        ] {
            assert_eq!(p(&[junk]).max_age, None, "{junk}");
        }
        assert_eq!(p(&["no-store-ish"]), CacheControl::default());
        assert_eq!(p(&[]), CacheControl::default());
    }

    #[test]
    fn storage_decision_follows_the_rules() {
        let none = CacheControl::default();
        let go =
            |cc: &CacheControl, vary, auth, allow| storage_decision(200, cc, vary, auth, allow);
        assert_eq!(go(&none, None, false, false), Ok(()));
        assert_eq!(
            storage_decision(404, &none, None, false, false),
            Err(NotStored::Status)
        );
        assert_eq!(
            storage_decision(304, &none, None, false, false),
            Err(NotStored::Status)
        );
        let no_store = CacheControl::parse(&["no-store"]);
        assert_eq!(go(&no_store, None, false, true), Err(NotStored::NoStore));
        let private = CacheControl::parse(&["private"]);
        assert_eq!(go(&private, None, false, false), Err(NotStored::Private));
        assert_eq!(go(&private, None, false, true), Ok(()));
        assert_eq!(go(&none, None, true, false), Err(NotStored::Authenticated));
        assert_eq!(go(&none, None, true, true), Ok(()));
        assert_eq!(
            go(&none, Some("Accept-Encoding, *"), false, true),
            Err(NotStored::VaryStar)
        );
        assert_eq!(go(&none, Some("Accept-Encoding"), false, false), Ok(()));
    }

    #[test]
    fn retry_after_accepts_seconds_and_http_dates_only() {
        assert_eq!(parse_retry_after("120", 0), Some(120));
        assert_eq!(parse_retry_after(" 7 ", 0), Some(7));
        assert_eq!(parse_retry_after("0", 0), Some(0));
        assert_eq!(parse_retry_after("99999999999999999999", 0), Some(u64::MAX));
        // 1994-11-06T08:49:37Z is 784111777.
        assert_eq!(
            parse_retry_after("Sun, 06 Nov 1994 08:49:37 GMT", 784_111_700),
            Some(77)
        );
        assert_eq!(
            parse_retry_after("Sun, 06 Nov 1994 08:49:37 GMT", 784_111_777),
            Some(0)
        );
        assert_eq!(
            parse_retry_after("Sun, 06 Nov 1994 08:49:37 GMT", 900_000_000),
            Some(0)
        );
        assert_eq!(
            parse_retry_after("Thu, 01 Jan 1970 00:00:00 GMT", 0),
            Some(0)
        );
        assert_eq!(
            parse_retry_after("Thu, 29 Feb 2024 12:00:00 GMT", 1_709_208_000 - 30),
            Some(30)
        );
        for bad in [
            "",
            "-5",
            "1.5",
            "5s",
            "1e3",
            "Sunday, 06-Nov-94 08:49:37 GMT",
            "Sun Nov  6 08:49:37 1994",
            "Sun, 31 Feb 2023 00:00:00 GMT",
            "Sun, 06 Nov 1994 25:00:00 GMT",
            "Xyz, 06 Nov 1994 08:49:37 GMT",
            "Sun, 06 Nov 1994 08:49:37 UTC",
            "Sun, 06 Nov 1969 00:00:00 GMT",
            "Sun, 00 Nov 1994 00:00:00 GMT",
        ] {
            assert_eq!(parse_retry_after(bad, 0), None, "{bad:?}");
        }
    }
}
