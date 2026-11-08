//! The opt-in persistent cache: conditional requests, what is and is not
//! stored, offline mode, and what happens to damaged or forged entries.

use std::path::{Path, PathBuf};
use std::time::Duration;

use libtea_client::{
    cache_clear, cache_stat, ApiKey, ArtifactFetch, CacheConfig, Client, ClientBuilder,
    ClientError, FileDownloadOptions, Limits, Service,
};
use libtea_core::http_cache::{cache_key, encode_entry, CacheEntryMeta};
use libtea_core::{
    origin_key, ArtifactId, Checksum, ChecksumAlgorithm, DestinationPolicy, IntegrityOutcome,
    ProductReleaseId,
};
use sha2::{Digest, Sha256};
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REL: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const ART: &str = "11111111-1111-4111-8111-111111111111";
const SECRET: &str = "hunter2-secret-value";

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("libtea-cache-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// A cache directory that does not exist yet, so the client creates it private.
    fn cache(&self) -> PathBuf {
        self.0.join("cache")
    }

    fn entries(&self) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = match std::fs::read_dir(self.cache().join("v1")) {
            Ok(read) => read.flatten().map(|e| e.path()).collect(),
            Err(_) => Vec::new(),
        };
        found.sort();
        found
    }

    fn entry_files(&self) -> Vec<PathBuf> {
        self.entries()
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "entry"))
            .collect()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sha256(bytes: &[u8]) -> Checksum {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Checksum::new(ChecksumAlgorithm::Sha256, &digest).unwrap()
}

fn release_body() -> serde_json::Value {
    serde_json::json!({
        "uuid": REL, "product": REL, "version": "1.0.0",
        "createdDate": "2024-01-01T00:00:00Z", "components": [],
        "latestCollection": {"uuid": REL, "version": 1, "createdDate": "2024-01-01T00:00:00Z",
            "belongsTo": "PRODUCT_RELEASE", "updateReason": {"type": "INITIAL_RELEASE"}, "artifacts": []}
    })
}

fn builder(server: &MockServer) -> ClientBuilder {
    Client::builder()
        .limits(
            Limits::default()
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(0),
        )
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap(),
        )
}

fn service(client: &Client, server: &MockServer) -> Service {
    client.service(&server.uri(), "1.0.0").unwrap()
}

fn release_path() -> String {
    format!("/v1.0.0/productRelease/{REL}")
}

async fn get_release(service: &Service) -> Result<(), ClientError> {
    service
        .get_product_release(ProductReleaseId::parse(REL).unwrap())
        .await
        .map(|_| ())
}

async fn request_headers(server: &MockServer) -> Vec<wiremock::http::HeaderMap> {
    server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.headers)
        .collect()
}

/// Mount a 200 for the release with the given extra headers, replacing mocks.
async fn serve_release(server: &MockServer, headers: &[(&str, &str)]) {
    server.reset().await;
    let mut response = ResponseTemplate::new(200).set_body_json(release_body());
    for (name, value) in headers {
        response = response.insert_header(*name, *value);
    }
    Mock::given(method("GET"))
        .and(path(release_path()))
        .respond_with(response)
        .mount(server)
        .await;
}

fn artifact_url(server: &MockServer) -> Url {
    Url::parse(&format!(
        "{}/v1.0.0/artifact/{ART}/1/download",
        server.uri()
    ))
    .unwrap()
}

fn key_for(url: &Url, key_id: Option<&str>) -> String {
    cache_key("GET", url.as_str(), &origin_key(url).unwrap(), key_id)
}

fn plant(dir: &Dir, key: &str, body: &[u8], etag: Option<&str>, validated_at: u64) {
    plant_typed(
        dir,
        key,
        body,
        etag,
        validated_at,
        "application/octet-stream",
    );
}

fn plant_typed(
    dir: &Dir,
    key: &str,
    body: &[u8],
    etag: Option<&str>,
    validated_at: u64,
    content_type: &str,
) {
    let mut headers = vec![("content-type".to_owned(), content_type.to_owned())];
    if let Some(etag) = etag {
        headers.push(("etag".to_owned(), etag.to_owned()));
    }
    let mut meta = CacheEntryMeta::new(key, "http://x/", 200, validated_at, true, &headers);
    meta.validated_at = validated_at;
    let bytes = encode_entry(meta, body).unwrap();
    let entries = dir.cache().join("v1");
    std::fs::create_dir_all(&entries).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.cache(), std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::set_permissions(&entries, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    std::fs::write(entries.join(format!("{key}.entry")), bytes).unwrap();
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ---------------------------------------------------------------- revalidation

#[tokio::test]
async fn a_cached_response_is_revalidated_with_its_etag_and_served_on_304() {
    let server = MockServer::start().await;
    let dir = Dir::new("etag");
    Mock::given(method("GET"))
        .and(path(release_path()))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304).insert_header("etag", "\"v1\""))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(release_path()))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"v1\"")
                .set_body_json(release_body()),
        )
        .with_priority(5)
        .mount(&server)
        .await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let service = service(&client, &server);
    get_release(&service).await.unwrap();
    assert_eq!(dir.entry_files().len(), 1);
    get_release(&service).await.unwrap();

    let seen = request_headers(&server).await;
    assert_eq!(seen.len(), 2);
    assert!(!seen[0].contains_key("if-none-match"));
    assert_eq!(
        seen[1].get("if-none-match").unwrap().to_str().unwrap(),
        "\"v1\""
    );
}

#[tokio::test]
async fn last_modified_is_sent_as_if_modified_since() {
    let server = MockServer::start().await;
    let dir = Dir::new("lm");
    serve_release(
        &server,
        &[("last-modified", "Sun, 06 Nov 1994 08:49:37 GMT")],
    )
    .await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let service = service(&client, &server);
    get_release(&service).await.unwrap();
    get_release(&service).await.unwrap();
    let seen = request_headers(&server).await;
    assert_eq!(
        seen[1].get("if-modified-since").unwrap().to_str().unwrap(),
        "Sun, 06 Nov 1994 08:49:37 GMT"
    );
}

#[tokio::test]
async fn a_changed_resource_replaces_the_entry() {
    let server = MockServer::start().await;
    let dir = Dir::new("replace");
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let service = service(&client, &server);
    get_release(&service).await.unwrap();
    let first = std::fs::read(&dir.entry_files()[0]).unwrap();
    serve_release(&server, &[("etag", "\"v2\"")]).await;
    get_release(&service).await.unwrap();
    let second = std::fs::read(&dir.entry_files()[0]).unwrap();
    assert_ne!(first, second);
    let text = String::from_utf8_lossy(&second).into_owned();
    // The ETag appears JSON-escaped in the stored metadata.
    assert!(text.contains("\\\"v2\\\"") && !text.contains("\\\"v1\\\""));
    assert_eq!(dir.entry_files().len(), 1);
}

#[tokio::test]
async fn a_404_removes_a_stale_entry() {
    let server = MockServer::start().await;
    let dir = Dir::new("gone");
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let service = service(&client, &server);
    get_release(&service).await.unwrap();
    assert_eq!(dir.entry_files().len(), 1);
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    assert!(get_release(&service).await.is_err());
    assert!(dir.entry_files().is_empty());
}

// ------------------------------------------------------- what is not stored

#[tokio::test]
async fn no_store_is_not_stored_and_evicts_an_existing_entry() {
    let server = MockServer::start().await;
    let dir = Dir::new("nostore");
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let service = service(&client, &server);
    get_release(&service).await.unwrap();
    assert_eq!(dir.entry_files().len(), 1);
    serve_release(
        &server,
        &[
            ("etag", "\"v2\""),
            ("cache-control", "max-age=60, No-Store"),
        ],
    )
    .await;
    get_release(&service).await.unwrap();
    assert!(
        dir.entry_files().is_empty(),
        "no-store must evict and not store"
    );
}

#[tokio::test]
async fn private_responses_need_an_explicit_opt_in() {
    let server = MockServer::start().await;
    let dir = Dir::new("private");
    serve_release(&server, &[("etag", "\"v1\""), ("cache-control", "private")]).await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    assert!(dir.entry_files().is_empty());

    let opted = Dir::new("private-opt");
    let client = builder(&server)
        .cache(CacheConfig::new(opted.cache()).with_authenticated(true))
        .build()
        .unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    assert_eq!(opted.entry_files().len(), 1);
}

#[tokio::test]
async fn vary_star_and_unlisted_headers_are_not_kept() {
    let server = MockServer::start().await;
    let dir = Dir::new("vary");
    serve_release(&server, &[("etag", "\"v1\""), ("vary", "*")]).await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    assert!(dir.entry_files().is_empty());

    serve_release(
        &server,
        &[
            ("etag", "\"v1\""),
            ("set-cookie", "session=topsecret"),
            ("x-internal", "do-not-keep"),
            ("authorization", "Bearer leaked"),
        ],
    )
    .await;
    get_release(&service(&client, &server)).await.unwrap();
    let stored = std::fs::read(&dir.entry_files()[0]).unwrap();
    let text = String::from_utf8_lossy(&stored);
    for forbidden in [
        "topsecret",
        "set-cookie",
        "do-not-keep",
        "leaked",
        "authorization",
    ] {
        assert!(!text.contains(forbidden), "{forbidden} reached the cache");
    }
}

#[tokio::test]
async fn hostile_header_values_are_dropped_not_trusted() {
    let server = MockServer::start().await;
    let dir = Dir::new("hostile");
    let huge = format!("\"{}\"", "e".repeat(9000));
    serve_release(
        &server,
        &[
            ("etag", huge.as_str()),
            (
                "cache-control",
                "max-age=99999999999999999999999, junk=\"; no-store x",
            ),
        ],
    )
    .await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let service = service(&client, &server);
    get_release(&service).await.unwrap();
    get_release(&service).await.unwrap();
    // The 9 KiB validator was not kept, so the second request was not conditional.
    let seen = request_headers(&server).await;
    assert!(!seen[1].contains_key("if-none-match"));
    for entry in dir.entry_files() {
        assert!(std::fs::metadata(entry).unwrap().len() < 4096);
    }
}

// ----------------------------------------------------------- authentication

async fn auth_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"tok","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(release_path()))
        .and(header("authorization", "Bearer tok"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"auth\"")
                .set_body_json(release_body()),
        )
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(release_path()))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .with_priority(5)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn responses_to_credentialed_requests_are_not_stored_unless_enabled() {
    let server = auth_server().await;
    let dir = Dir::new("auth-off");
    let client = builder(&server)
        .api_key(ApiKey::new("key-a", SECRET))
        .cache_dir(dir.cache())
        .build()
        .unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    assert!(
        dir.entry_files().is_empty(),
        "an authenticated response was stored"
    );

    let on = Dir::new("auth-on");
    let client = builder(&server)
        .api_key(ApiKey::new("key-a", SECRET))
        .cache(CacheConfig::new(on.cache()).with_authenticated(true))
        .build()
        .unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    assert_eq!(on.entry_files().len(), 1);
    for entry in on.entries() {
        let text = String::from_utf8_lossy(&std::fs::read(entry).unwrap()).into_owned();
        assert!(
            !text.contains(SECRET) && !text.contains("tok\""),
            "a credential reached the cache"
        );
        assert!(!text.contains("key-a"), "the key id is only ever hashed");
    }
}

#[tokio::test]
async fn different_keys_never_share_entries() {
    let server = auth_server().await;
    let dir = Dir::new("keys");
    let config = || CacheConfig::new(dir.cache()).with_authenticated(true);
    let one = builder(&server)
        .api_key(ApiKey::new("key-a", SECRET))
        .cache(config())
        .build()
        .unwrap();
    get_release(&service(&one, &server)).await.unwrap();
    assert_eq!(dir.entry_files().len(), 1);
    let two = builder(&server)
        .api_key(ApiKey::new("key-b", SECRET))
        .cache(config())
        .build()
        .unwrap();
    get_release(&service(&two, &server)).await.unwrap();
    assert_eq!(
        dir.entry_files().len(),
        2,
        "key-b must not reuse key-a's entry"
    );
    // key-b's first request carried no validator: it found nothing under its own key.
    let first_of_b = request_headers(&server)
        .await
        .into_iter()
        .filter(|h| h.contains_key("authorization") || !h.contains_key("if-none-match"))
        .count();
    assert!(first_of_b >= 2);
}

// ----------------------------------------------------------- forged entries

#[tokio::test]
async fn a_forged_artifact_entry_is_caught_by_the_checksum_check() {
    let server = MockServer::start().await;
    let dir = Dir::new("forged");
    let url = artifact_url(&server);
    // Valid format, valid self-hash, wrong content, and a validator the server accepts.
    plant(
        &dir,
        &key_for(&url, None),
        b"forged bytes",
        Some("\"v1\""),
        now(),
    );
    Mock::given(method("GET"))
        .and(path(url.path()))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;
    let good = b"genuine bytes";
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let error = service(&client, &server)
        .download_artifact(
            ArtifactId::parse(ART).unwrap(),
            1,
            None,
            &[sha256(good)],
            None,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            ClientError::Integrity {
                outcome: IntegrityOutcome::Failed,
                ..
            }
        ),
        "{error:?}"
    );

    // A failed check evicts the entry (see the eviction tests), so plant it
    // again: offline, the same forged entry meets the same check, buffered
    // and streaming.
    plant(
        &dir,
        &key_for(&url, None),
        b"forged bytes",
        Some("\"v1\""),
        now(),
    );
    let offline = builder(&server)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    let svc = service(&offline, &server);
    let error = svc
        .download_artifact(
            ArtifactId::parse(ART).unwrap(),
            1,
            None,
            &[sha256(good)],
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Integrity { .. }), "{error:?}");
    plant(
        &dir,
        &key_for(&url, None),
        b"forged bytes",
        Some("\"v1\""),
        now(),
    );
    let out = dir.0.join("out.bin");
    let error = svc
        .download_artifact_to_file(
            ArtifactId::parse(ART).unwrap(),
            1,
            None,
            &[sha256(good)],
            &out,
            &FileDownloadOptions::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Integrity { .. }), "{error:?}");
    assert!(
        !out.exists(),
        "forged bytes must not be committed to a file"
    );
}

#[tokio::test]
async fn a_cached_artifact_that_matches_is_served_and_still_verified() {
    let server = MockServer::start().await;
    let dir = Dir::new("genuine");
    let url = artifact_url(&server);
    let body = b"genuine bytes";
    plant(&dir, &key_for(&url, None), body, Some("\"v1\""), now());
    let offline = builder(&server)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    let fetched = service(&offline, &server)
        .download_artifact(
            ArtifactId::parse(ART).unwrap(),
            1,
            None,
            &[sha256(body)],
            None,
        )
        .await
        .unwrap();
    let ArtifactFetch::Body(downloaded) = fetched else {
        panic!("expected a body");
    };
    assert_eq!(downloaded.bytes, body);
    assert_eq!(downloaded.integrity.outcome, IntegrityOutcome::Verified);
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "offline must not connect"
    );
}

// ------------------------------------------------------ damaged entries

#[tokio::test]
async fn damaged_entries_are_treated_as_absent_and_removed() {
    let server = MockServer::start().await;
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let url = Url::parse(&format!("{}{}", server.uri(), release_path())).unwrap();
    let key = key_for(&url, None);

    let make_good = |dir: &Dir| builder_sync(&server, dir);
    fn builder_sync(server: &MockServer, dir: &Dir) -> Client {
        builder(server).cache_dir(dir.cache()).build().unwrap()
    }
    // Produce a genuine entry once, then damage copies of it in different ways.
    let seed = Dir::new("dmg-seed");
    get_release(&service(&make_good(&seed), &server))
        .await
        .unwrap();
    let genuine = std::fs::read(&seed.entry_files()[0]).unwrap();
    let other_key = "f".repeat(64);

    let mut flipped = genuine.clone();
    flipped[20] ^= 0x01;
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", Vec::new()),
        ("garbage", vec![0xff; 600]),
        ("truncated", genuine[..genuine.len() / 2].to_vec()),
        ("flipped body byte", flipped),
        ("only the magic", b"LIBTEA-CACHE 1\n".to_vec()),
        ("huge", vec![b'x'; 3 * 1024 * 1024]),
    ];
    for (label, bytes) in cases {
        let dir = Dir::new("dmg");
        plant(&dir, &key, b"placeholder", None, now());
        let path = dir.cache().join("v1").join(format!("{key}.entry"));
        std::fs::write(&path, &bytes).unwrap();
        server.reset().await;
        serve_release(&server, &[("etag", "\"v1\"")]).await;
        let client = builder_sync(&server, &dir);
        get_release(&service(&client, &server))
            .await
            .unwrap_or_else(|e| panic!("{label}: {e:?}"));
        // The request went to the network and the entry was rewritten correctly.
        let seen = request_headers(&server).await;
        assert_eq!(seen.len(), 1, "{label}");
        assert!(
            !seen[0].contains_key("if-none-match"),
            "{label}: a bad entry gave a validator"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            genuine_after(&path),
            "{label}"
        );
    }

    // An entry stored under another key is refused.
    let dir = Dir::new("dmg-key");
    plant(&dir, &other_key, b"x", None, now());
    let moved = dir.cache().join("v1").join(format!("{key}.entry"));
    std::fs::rename(
        dir.cache().join("v1").join(format!("{other_key}.entry")),
        &moved,
    )
    .unwrap();
    let client = builder_sync(&server, &dir);
    get_release(&service(&client, &server)).await.unwrap();
    assert_eq!(request_headers(&server).await.len(), 2);

    fn genuine_after(path: &Path) -> Vec<u8> {
        std::fs::read(path).unwrap()
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_symlink_in_place_of_an_entry_is_removed_and_never_followed() {
    let server = MockServer::start().await;
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let url = Url::parse(&format!("{}{}", server.uri(), release_path())).unwrap();
    let key = key_for(&url, None);
    let dir = Dir::new("symlink-entry");
    plant(&dir, &key, b"x", None, now());
    let target = dir.0.join("precious.txt");
    std::fs::write(&target, b"do not touch").unwrap();
    let entry = dir.cache().join("v1").join(format!("{key}.entry"));
    std::fs::remove_file(&entry).unwrap();
    std::os::unix::fs::symlink(&target, &entry).unwrap();
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"do not touch");
    assert!(!std::fs::symlink_metadata(&entry)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[tokio::test]
async fn a_directory_where_an_entry_belongs_does_not_break_requests() {
    let server = MockServer::start().await;
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let url = Url::parse(&format!("{}{}", server.uri(), release_path())).unwrap();
    let key = key_for(&url, None);
    let dir = Dir::new("dir-entry");
    plant(&dir, &key, b"x", None, now());
    let entry = dir.cache().join("v1").join(format!("{key}.entry"));
    std::fs::remove_file(&entry).unwrap();
    std::fs::create_dir(&entry).unwrap();
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    get_release(&service(&client, &server)).await.unwrap();
}

// ------------------------------------------------------ directory safety

#[cfg(unix)]
#[tokio::test]
async fn unsafe_cache_directories_are_refused() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let server = MockServer::start().await;
    let dir = Dir::new("unsafe");
    let build = |p: PathBuf| builder(&server).cache_dir(p).build();

    // A symbolic link: refused, and nothing is written through it.
    let real = dir.0.join("real");
    std::fs::create_dir(&real).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o700)).unwrap();
    let link = dir.0.join("link");
    symlink(&real, &link).unwrap();
    match build(link) {
        Err(ClientError::Io(text)) => assert!(text.contains("symbolic link"), "{text}"),
        Err(other) => panic!("{other:?}"),
        Ok(_) => panic!("a symlinked cache directory was accepted"),
    }
    assert_eq!(std::fs::read_dir(&real).unwrap().count(), 0);

    // Open to group or others: refused.
    for mode in [0o755, 0o750, 0o770, 0o707] {
        let open = dir.0.join(format!("open-{mode:o}"));
        std::fs::create_dir(&open).unwrap();
        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(mode)).unwrap();
        match build(open) {
            Err(ClientError::Io(text)) => assert!(text.contains("chmod 700"), "{text}"),
            other => panic!("mode {mode:o}: {:?}", other.map(|_| ())),
        }
    }

    // A file where the directory should be.
    let file = dir.0.join("afile");
    std::fs::write(&file, b"x").unwrap();
    assert!(matches!(build(file), Err(ClientError::Io(_))));

    // A symlinked v1 inside an otherwise fine directory.
    let fine = dir.0.join("fine");
    std::fs::create_dir(&fine).unwrap();
    std::fs::set_permissions(&fine, std::fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&real, fine.join("v1")).unwrap();
    assert!(matches!(build(fine), Err(ClientError::Io(_))));
    assert_eq!(std::fs::read_dir(&real).unwrap().count(), 0);
}

#[cfg(unix)]
#[tokio::test]
async fn the_cache_is_created_private() {
    use std::os::unix::fs::PermissionsExt;
    let server = MockServer::start().await;
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let dir = Dir::new("modes");
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&dir.cache()), 0o700);
    assert_eq!(mode(&dir.cache().join("v1")), 0o700);
    assert_eq!(mode(&dir.entry_files()[0]), 0o600);
    // An existing private directory is used as it is.
    assert!(builder(&server).cache_dir(dir.cache()).build().is_ok());
}

// ------------------------------------------------------------- limits

#[tokio::test]
async fn age_and_size_limits_hold() {
    let server = MockServer::start().await;
    let url = artifact_url(&server);

    // An entry older than max_age is not used, even offline.
    let dir = Dir::new("age");
    plant(&dir, &key_for(&url, None), b"old", None, now() - 10_000);
    let offline = builder(&server)
        .cache(CacheConfig::new(dir.cache()).with_max_age(Duration::from_secs(60)))
        .offline(true)
        .build()
        .unwrap();
    let error = service(&offline, &server)
        .download_artifact(ArtifactId::parse(ART).unwrap(), 1, None, &[], None)
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Offline), "{error:?}");
    assert!(dir.entry_files().is_empty(), "an expired entry is removed");

    // A future-dated entry is distrusted.
    let dir = Dir::new("future");
    plant(
        &dir,
        &key_for(&url, None),
        b"future",
        None,
        now() + 10 * 86_400,
    );
    let offline = builder(&server)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    assert!(matches!(
        service(&offline, &server)
            .download_artifact(ArtifactId::parse(ART).unwrap(), 1, None, &[], None)
            .await,
        Err(ClientError::Offline)
    ));

    // A body larger than max_entry_bytes is simply not cached.
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let dir = Dir::new("entry-size");
    let client = builder(&server)
        .cache(CacheConfig::new(dir.cache()).with_max_entry_bytes(10))
        .build()
        .unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    assert!(dir.entry_files().is_empty());

    // The total is bounded and the oldest entries go first.
    let dir = Dir::new("total");
    server.reset().await;
    for version in 1..=5 {
        Mock::given(method("GET"))
            .and(path(format!("/v1.0.0/artifact/{ART}/{version}/download")))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("etag", "\"e\"")
                    .set_body_raw(vec![b'a'; 1000], "application/octet-stream"),
            )
            .mount(&server)
            .await;
    }
    // Room for two entries (about 1.6 KB each with their metadata), not three.
    let client = builder(&server)
        .cache(CacheConfig::new(dir.cache()).with_max_total_bytes(3500))
        .build()
        .unwrap();
    for version in 1..=5 {
        service(&client, &server)
            .download_artifact(ArtifactId::parse(ART).unwrap(), version, None, &[], None)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    let files = dir.entry_files();
    let total: u64 = files
        .iter()
        .map(|p| std::fs::metadata(p).unwrap().len())
        .sum();
    assert!(total <= 3500, "cache grew to {total} bytes");
    assert!(
        (1..=2).contains(&files.len()),
        "{} entries kept",
        files.len()
    );
    let key_of = |version: u64| {
        let url = Url::parse(&format!(
            "{}/v1.0.0/artifact/{ART}/{version}/download",
            server.uri()
        ))
        .unwrap();
        format!("{}.entry", key_for(&url, None))
    };
    let kept: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(kept.contains(&key_of(5)), "the newest entry must survive");
    assert!(
        !kept.contains(&key_of(1)),
        "the oldest entry must be pruned first"
    );
}

// --------------------------------------------------------------- offline

#[tokio::test]
async fn offline_serves_from_the_cache_and_misses_are_an_error() {
    let server = MockServer::start().await;
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let dir = Dir::new("offline");
    let online = builder(&server).cache_dir(dir.cache()).build().unwrap();
    get_release(&service(&online, &server)).await.unwrap();
    let requests_before = server.received_requests().await.unwrap().len();

    let offline = builder(&server)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    let svc = service(&offline, &server);
    get_release(&svc).await.unwrap();
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        requests_before
    );

    // A different resource is a miss.
    let miss = svc
        .get_product_release(
            ProductReleaseId::parse("4cc5e0f3-5d6f-4d1a-8e5a-0e9d2f4a9b11").unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(miss, ClientError::Offline), "{miss:?}");
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        requests_before
    );
}

#[tokio::test]
async fn offline_needs_a_cache() {
    let server = MockServer::start().await;
    assert!(matches!(
        builder(&server).offline(true).build(),
        Err(ClientError::Model(_))
    ));
}

// ------------------------------------------------------------- streaming

#[tokio::test]
async fn a_streamed_download_is_cached_only_after_it_verifies() {
    let server = MockServer::start().await;
    let dir = Dir::new("stream");
    let url = artifact_url(&server);
    let good = vec![7u8; 200_000];
    let out = dir.0.join("out.bin");
    let id = ArtifactId::parse(ART).unwrap();

    // Tampered body: fails, leaves no entry and no temporary file.
    Mock::given(method("GET"))
        .and(path(url.path()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(b"tampered".to_vec(), "application/octet-stream"),
        )
        .mount(&server)
        .await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let svc = service(&client, &server);
    let error = svc
        .download_artifact_to_file(
            id,
            1,
            None,
            &[sha256(&good)],
            &out,
            &FileDownloadOptions::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Integrity { .. }));
    assert!(dir.entries().is_empty(), "left behind: {:?}", dir.entries());

    // Genuine body: stored, and a later offline run reproduces it.
    server.reset().await;
    Mock::given(method("GET"))
        .and(path(url.path()))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"s1\"")
                .set_body_raw(good.clone(), "application/octet-stream"),
        )
        .mount(&server)
        .await;
    svc.download_artifact_to_file(
        id,
        1,
        None,
        &[sha256(&good)],
        &out,
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap();
    assert_eq!(dir.entry_files().len(), 1);
    assert_eq!(dir.entries().len(), 1, "no temporary file may remain");

    let offline = builder(&server)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    let again = dir.0.join("again.bin");
    let result = service(&offline, &server)
        .download_artifact_to_file(
            id,
            1,
            None,
            &[sha256(&good)],
            &again,
            &FileDownloadOptions::new(),
        )
        .await
        .unwrap();
    assert_eq!(std::fs::read(&again).unwrap(), good);
    assert_eq!(
        result.download.integrity.outcome,
        IntegrityOutcome::Verified
    );
    assert_eq!(result.download.etag.as_deref(), Some("\"s1\""));

    // Online again: a conditional request, answered 304, served from the entry.
    server.reset().await;
    Mock::given(method("GET"))
        .and(path(url.path()))
        .and(header("if-none-match", "\"s1\""))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;
    let third = dir.0.join("third.bin");
    svc.download_artifact_to_file(
        id,
        1,
        None,
        &[sha256(&good)],
        &third,
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&third).unwrap(), good);
}

// ------------------------------------------------------ concurrency, tools

#[tokio::test]
async fn concurrent_clients_never_leave_a_corrupt_entry() {
    let server = MockServer::start().await;
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let dir = Dir::new("race");
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
        let svc = service(&client, &server);
        tasks.push(tokio::spawn(async move {
            for _ in 0..5 {
                get_release(&svc).await.unwrap();
            }
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
    assert_eq!(dir.entry_files().len(), 1);
    assert_eq!(
        dir.entries().len(),
        1,
        "temporary files left: {:?}",
        dir.entries()
    );
    let offline = builder(&server)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    get_release(&service(&offline, &server)).await.unwrap();
}

#[tokio::test]
async fn stat_and_clear_touch_only_cache_entries() {
    let server = MockServer::start().await;
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let dir = Dir::new("tools");
    let missing = cache_stat(&dir.cache()).unwrap();
    assert!(!missing.exists && missing.entries == 0);

    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    std::fs::write(dir.cache().join("v1").join("notes.txt"), b"mine").unwrap();
    let stat = cache_stat(&dir.cache()).unwrap();
    assert!(stat.exists);
    assert_eq!(stat.entries, 1);
    assert_eq!(stat.foreign, 1);
    assert!(stat.bytes > 0);

    #[cfg(unix)]
    {
        let target = dir.0.join("precious.txt");
        std::fs::write(&target, b"keep").unwrap();
        let name = format!("{}.entry", "e".repeat(64));
        std::os::unix::fs::symlink(&target, dir.cache().join("v1").join(name)).unwrap();
    }
    let removed = cache_clear(&dir.cache()).unwrap();
    assert!(removed >= 1);
    assert!(dir.entry_files().is_empty());
    assert_eq!(
        std::fs::read(dir.cache().join("v1").join("notes.txt")).unwrap(),
        b"mine"
    );
    #[cfg(unix)]
    assert_eq!(std::fs::read(dir.0.join("precious.txt")).unwrap(), b"keep");
}

// ------------------------------------------- A3: hits obey limits and policy

fn entry_meta_of(dir: &Dir, key: &str) -> libtea_core::http_cache::CacheEntryMeta {
    let bytes = std::fs::read(dir.cache().join("v1").join(format!("{key}.entry"))).unwrap();
    libtea_core::http_cache::decode_entry(&bytes, key, 1 << 24)
        .unwrap()
        .0
}

#[tokio::test]
async fn a_cache_hit_obeys_the_current_size_limit_for_json() {
    let server = MockServer::start().await;
    let dir = Dir::new("hit-json-size");
    let url = Url::parse(&format!("{}{}", server.uri(), release_path())).unwrap();
    // Stored under a larger limit (or planted): 3000 bytes against a 1000-byte limit.
    plant(
        &dir,
        &key_for(&url, None),
        &vec![b'x'; 3000],
        Some("\"v1\""),
        now(),
    );
    let limits = Limits::default()
        .with_initial_backoff(Duration::ZERO)
        .with_max_retries(0)
        .with_max_json_bytes(1000);
    let offline = builder(&server)
        .limits(limits)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    let error = get_release(&service(&offline, &server)).await.unwrap_err();
    assert!(matches!(error, ClientError::TooLarge), "{error:?}");
    assert!(
        dir.entry_files().is_empty(),
        "an entry over the limit is dropped"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn an_oversize_entry_is_ignored_online_and_the_server_is_asked_afresh() {
    let server = MockServer::start().await;
    let dir = Dir::new("hit-online-size");
    let url = artifact_url(&server);
    plant(
        &dir,
        &key_for(&url, None),
        &vec![b'x'; 5000],
        Some("\"old\""),
        now(),
    );
    Mock::given(method("GET"))
        .and(path(url.path()))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"new\"")
                .set_body_raw(vec![b'y'; 500], "application/octet-stream"),
        )
        .mount(&server)
        .await;
    let limits = Limits::default()
        .with_initial_backoff(Duration::ZERO)
        .with_max_retries(0)
        .with_max_artifact_bytes(1000);
    let client = builder(&server)
        .limits(limits)
        .cache_dir(dir.cache())
        .build()
        .unwrap();
    let fetched = service(&client, &server)
        .download_artifact(ArtifactId::parse(ART).unwrap(), 1, None, &[], None)
        .await
        .unwrap();
    let ArtifactFetch::Body(body) = fetched else {
        panic!("expected a body");
    };
    assert_eq!(body.bytes.len(), 500);
    let seen = request_headers(&server).await;
    assert_eq!(seen.len(), 1);
    assert!(
        !seen[0].contains_key("if-none-match"),
        "an entry over the limit must not supply a validator"
    );
}

#[tokio::test]
async fn a_streamed_cache_hit_obeys_the_size_limit_too() {
    let server = MockServer::start().await;
    let dir = Dir::new("hit-stream-size");
    let url = artifact_url(&server);
    plant(&dir, &key_for(&url, None), &vec![b'x'; 5000], None, now());
    let limits = Limits::default()
        .with_initial_backoff(Duration::ZERO)
        .with_max_retries(0)
        .with_max_artifact_bytes(1000);
    let offline = builder(&server)
        .limits(limits)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    let out = dir.0.join("out.bin");
    let error = service(&offline, &server)
        .download_artifact_to_file(
            ArtifactId::parse(ART).unwrap(),
            1,
            None,
            &[],
            &out,
            &FileDownloadOptions::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::TooLarge), "{error:?}");
    assert!(!out.exists());
}

/// An artifact whose only format is fetched from `external` (a different
/// origin from the API root).
fn external_artifact(external: &MockServer, checksum: &Checksum) -> libtea_core::Artifact {
    let body = serde_json::json!({
        "uuid": ART, "version": 1, "type": "BOM",
        "createdDate": "2024-01-01T00:00:00Z",
        "formats": [{
            "mediaType": "application/octet-stream",
            "url": format!("{}/v1.0.0/artifact/{ART}/1/download", external.uri()),
            "checksums": [{"algType": "SHA-256", "algValue": checksum.hex_value()}]
        }]
    });
    libtea_core::parse_artifact(body.to_string().as_bytes()).unwrap()
}

#[tokio::test]
async fn a_cache_hit_obeys_this_clients_destination_policy() {
    let api = MockServer::start().await;
    let external = MockServer::start().await; // never allowed by the client below
    let dir = Dir::new("hit-policy");
    let good = b"genuine bytes";
    let ext_url = Url::parse(&format!(
        "{}/v1.0.0/artifact/{ART}/1/download",
        external.uri()
    ))
    .unwrap();
    // Another client, with a permissive policy, stored it.
    plant(&dir, &key_for(&ext_url, None), good, Some("\"v1\""), now());
    let artifact = external_artifact(&external, &sha256(good));

    // This client may talk to `api` only.
    let strict = builder(&api)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    let svc = service(&strict, &api);
    let buffered = svc
        .download_format(&artifact, "application/octet-stream", None)
        .await;
    assert!(
        matches!(buffered, Err(ClientError::Policy(_))),
        "a cached response from a forbidden origin was served: {buffered:?}"
    );
    let mut sink: Vec<u8> = Vec::new();
    let streamed = svc
        .download_format_to_writer(&artifact, "application/octet-stream", &mut sink)
        .await;
    assert!(
        matches!(streamed, Err(ClientError::Policy(_))),
        "{streamed:?}"
    );
    assert!(sink.is_empty(), "no byte may reach the writer");

    // Control: with the origin allowed, the same entry is served.
    let permissive = Client::builder()
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&api.uri()).unwrap())
                .unwrap()
                .and_allow(&Url::parse(&external.uri()).unwrap())
                .unwrap(),
        )
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    let ok = service(&permissive, &api)
        .download_format(&artifact, "application/octet-stream", None)
        .await;
    assert!(matches!(ok, Ok(ArtifactFetch::Body(_))), "{ok:?}");
}

// ------------------------- A3: failed checks evict; age renews only after them

#[tokio::test]
async fn bytes_from_the_cache_that_fail_their_checksums_are_evicted() {
    let server = MockServer::start().await;
    let dir = Dir::new("evict");
    let url = artifact_url(&server);
    let key = key_for(&url, None);
    let good = b"genuine bytes";
    let id = ArtifactId::parse(ART).unwrap();
    let old = now() - 1000;

    // Online: the server confirms the forged entry with a 304.
    plant(&dir, &key, b"forged bytes", Some("\"v1\""), old);
    Mock::given(method("GET"))
        .and(path(url.path()))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(url.path()))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"v2\"")
                .set_body_raw(good.to_vec(), "application/octet-stream"),
        )
        .with_priority(5)
        .mount(&server)
        .await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let svc = service(&client, &server);
    let error = svc
        .download_artifact(id, 1, None, &[sha256(good)], None)
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Integrity { .. }), "{error:?}");
    assert!(
        dir.entry_files().is_empty(),
        "the forged entry must be gone, not renewed by the 304"
    );
    // The next try asks without a validator and gets the genuine bytes.
    let healed = svc
        .download_artifact(id, 1, None, &[sha256(good)], None)
        .await
        .unwrap();
    assert!(matches!(healed, ArtifactFetch::Body(_)));
    let seen = request_headers(&server).await;
    assert!(!seen.last().unwrap().contains_key("if-none-match"));
    assert_eq!(entry_meta_of(&dir, &key).header("etag"), Some("\"v2\""));

    // Offline, buffered.
    plant(&dir, &key, b"forged bytes", Some("\"v1\""), old);
    let offline = builder(&server)
        .cache_dir(dir.cache())
        .offline(true)
        .build()
        .unwrap();
    let error = service(&offline, &server)
        .download_artifact(id, 1, None, &[sha256(good)], None)
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Integrity { .. }), "{error:?}");
    assert!(dir.entry_files().is_empty());

    // Offline, streamed.
    plant(&dir, &key, b"forged bytes", Some("\"v1\""), old);
    let out = dir.0.join("out.bin");
    let error = service(&offline, &server)
        .download_artifact_to_file(
            id,
            1,
            None,
            &[sha256(good)],
            &out,
            &FileDownloadOptions::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Integrity { .. }), "{error:?}");
    assert!(dir.entry_files().is_empty());
    assert!(!out.exists());
}

#[tokio::test]
async fn a_streamed_304_with_forged_bytes_evicts_instead_of_renewing() {
    let server = MockServer::start().await;
    let dir = Dir::new("evict-stream");
    let url = artifact_url(&server);
    let good = b"genuine bytes";
    plant(
        &dir,
        &key_for(&url, None),
        b"forged bytes",
        Some("\"v1\""),
        now() - 1000,
    );
    Mock::given(method("GET"))
        .and(path(url.path()))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let out = dir.0.join("out.bin");
    let error = service(&client, &server)
        .download_artifact_to_file(
            ArtifactId::parse(ART).unwrap(),
            1,
            None,
            &[sha256(good)],
            &out,
            &FileDownloadOptions::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Integrity { .. }), "{error:?}");
    assert!(dir.entry_files().is_empty());
    assert!(!out.exists());
}

#[tokio::test]
async fn a_304_renews_the_entry_only_after_the_checksums_passed() {
    let server = MockServer::start().await;
    let dir = Dir::new("renew");
    let url = artifact_url(&server);
    let key = key_for(&url, None);
    let good = b"genuine bytes";
    let stale = now() - 3600;
    plant(&dir, &key, good, Some("\"v1\""), stale);
    Mock::given(method("GET"))
        .and(path(url.path()))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    let svc = service(&client, &server);
    let id = ArtifactId::parse(ART).unwrap();
    svc.download_artifact(id, 1, None, &[sha256(good)], None)
        .await
        .unwrap();
    let renewed = entry_meta_of(&dir, &key).validated_at;
    assert!(
        renewed >= now() - 5,
        "buffered: validated_at {renewed} was not renewed"
    );

    // Streamed.
    plant(&dir, &key, good, Some("\"v1\""), stale);
    let out = dir.0.join("out.bin");
    svc.download_artifact_to_file(
        id,
        1,
        None,
        &[sha256(good)],
        &out,
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap();
    assert!(
        entry_meta_of(&dir, &key).validated_at >= now() - 5,
        "streamed: not renewed"
    );
}

#[tokio::test]
async fn a_json_304_renews_the_entry_at_once() {
    let server = MockServer::start().await;
    let dir = Dir::new("renew-json");
    let url = Url::parse(&format!("{}{}", server.uri(), release_path())).unwrap();
    let key = key_for(&url, None);
    let body = serde_json::to_vec(&release_body()).unwrap();
    plant_typed(
        &dir,
        &key,
        &body,
        Some("\"r1\""),
        now() - 3600,
        "application/json",
    );
    Mock::given(method("GET"))
        .and(path(release_path()))
        .and(header("if-none-match", "\"r1\""))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    assert!(entry_meta_of(&dir, &key).validated_at >= now() - 5);
}

#[tokio::test]
async fn cache_clear_deletes_only_files_that_have_the_caches_temporary_shape() {
    let server = MockServer::start().await;
    serve_release(&server, &[("etag", "\"v1\"")]).await;
    let dir = Dir::new("clear-shape");
    let client = builder(&server).cache_dir(dir.cache()).build().unwrap();
    get_release(&service(&client, &server)).await.unwrap();
    let entries = dir.cache().join("v1");
    let key = "a".repeat(64);
    // The cache's own leftover (pid and counter after the key) and look-alikes.
    let own = format!(".{key}.4242-7.tmp");
    let keep = [
        ".mine.tmp".to_owned(),
        ".notes.libtea.tmp".to_owned(),
        "build.tmp".to_owned(),
        format!(".{key}.tmp"),
        format!(".{key}.x-7.tmp"),
        format!(".{key}.4242-7.tmp.bak"),
        "notes.txt".to_owned(),
    ];
    std::fs::write(entries.join(&own), b"stale").unwrap();
    for name in &keep {
        std::fs::write(entries.join(name), b"mine").unwrap();
    }
    let removed = cache_clear(&dir.cache()).unwrap();
    assert_eq!(removed, 2, "the entry and the cache's own temporary file");
    assert!(!entries.join(&own).exists());
    for name in &keep {
        assert_eq!(
            std::fs::read(entries.join(name)).unwrap(),
            b"mine",
            "{name} is not the cache's file and must survive"
        );
    }
}
