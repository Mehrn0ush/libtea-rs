//! Streaming downloads: incremental hashing, limits, and "nothing is released
//! before the checksum check completes" for the file API.

use std::path::{Path, PathBuf};
use std::time::Duration;

use libtea_client::{Client, ClientError, FileDownloadOptions, Limits, Service};
use libtea_core::{ArtifactId, Checksum, ChecksumAlgorithm, DestinationPolicy, IntegrityOutcome};
use sha2::{Digest, Sha256};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "libtea-stream-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sha256(bytes: &[u8]) -> Checksum {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Checksum::new(ChecksumAlgorithm::Sha256, &digest).unwrap()
}

fn gzip(raw: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, raw).unwrap();
    encoder.finish().unwrap()
}

/// An artifact whose only format is fetched from `server` through its `url`.
fn external_artifact(server: &MockServer, checksum: &Checksum) -> libtea_core::Artifact {
    let body = serde_json::json!({
        "uuid": ID,
        "version": 1,
        "type": "BOM",
        "createdDate": "2024-01-01T00:00:00Z",
        "formats": [{
            "mediaType": "application/octet-stream",
            "url": format!("{}/v1.0.0/artifact/{ID}/1/download", server.uri()),
            "checksums": [{"algType": "SHA-256", "algValue": checksum.hex_value()}]
        }]
    });
    libtea_core::parse_artifact(body.to_string().as_bytes()).unwrap()
}

fn allow(server: &MockServer) -> DestinationPolicy {
    DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap()
}

fn client_with(server: &MockServer, limits: Limits, allow_weak: bool) -> Client {
    Client::builder()
        .limits(
            limits
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(0),
        )
        .destination_policy(allow(server))
        .allow_weak_checksums(allow_weak)
        .build()
        .unwrap()
}

fn service(server: &MockServer) -> Service {
    client_with(server, Limits::default(), false)
        .service(&server.uri(), "1.0.0")
        .unwrap()
}

async fn serve(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(response)
        .mount(server)
        .await;
}

async fn to_file(
    service: &Service,
    checksums: &[Checksum],
    dest: &Path,
    options: &FileDownloadOptions,
) -> Result<libtea_client::FileDownload, ClientError> {
    service
        .download_artifact_to_file(
            ArtifactId::parse(ID).unwrap(),
            1,
            None,
            checksums,
            dest,
            options,
        )
        .await
}

#[tokio::test]
async fn a_multi_chunk_body_is_written_and_verified() {
    let raw: Vec<u8> = (0..3_000_000u32).map(|n| (n % 253) as u8).collect();
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200).set_body_raw(raw.clone(), "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("ok");
    let dest = dir.file("sbom.json");
    let result = to_file(
        &service(&server),
        &[sha256(&raw)],
        &dest,
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), raw);
    assert_eq!(result.download.bytes_written, raw.len() as u64);
    assert_eq!(
        result.download.integrity.outcome,
        IntegrityOutcome::Verified
    );
    assert_eq!(dir.names(), ["sbom.json"], "no temporary file may remain");
}

#[tokio::test]
async fn gzip_is_decoded_before_hashing_and_writing() {
    let raw = b"{\"bomFormat\":\"CycloneDX\"}".repeat(1000);
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200)
            .insert_header("content-encoding", "gzip")
            .set_body_raw(gzip(&raw), "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("gz");
    let dest = dir.file("out.json");
    let result = to_file(
        &service(&server),
        &[sha256(&raw)],
        &dest,
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), raw);
    assert_eq!(result.download.content_encoding.as_deref(), Some("gzip"));
    assert_eq!(
        result.download.integrity.outcome,
        IntegrityOutcome::Verified
    );
}

#[tokio::test]
async fn a_checksum_mismatch_releases_nothing() {
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200).set_body_raw("tampered", "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("bad");
    let dest = dir.file("out.json");
    let error = to_file(
        &service(&server),
        &[sha256(b"original")],
        &dest,
        &FileDownloadOptions::new(),
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
    assert!(dir.names().is_empty(), "left behind: {:?}", dir.names());
}

#[tokio::test]
async fn a_failed_download_leaves_an_existing_destination_untouched() {
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200).set_body_raw("tampered", "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("keep");
    let dest = dir.file("out.json");
    std::fs::write(&dest, b"previous good content").unwrap();
    let options = FileDownloadOptions::new().with_overwrite(true);
    to_file(&service(&server), &[sha256(b"original")], &dest, &options)
        .await
        .unwrap_err();
    assert_eq!(std::fs::read(&dest).unwrap(), b"previous good content");
    assert_eq!(dir.names(), ["out.json"]);
}

#[tokio::test]
async fn an_existing_destination_needs_overwrite() {
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200).set_body_raw("tea", "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("exists");
    let dest = dir.file("out.json");
    std::fs::write(&dest, b"old").unwrap();
    let error = to_file(
        &service(&server),
        &[sha256(b"tea")],
        &dest,
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, ClientError::Io(_)), "{error:?}");
    assert_eq!(std::fs::read(&dest).unwrap(), b"old");
    to_file(
        &service(&server),
        &[sha256(b"tea")],
        &dest,
        &FileDownloadOptions::new().with_overwrite(true),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"tea");
    assert_eq!(dir.names(), ["out.json"]);
}

#[cfg(unix)]
#[tokio::test]
async fn a_symlink_destination_is_refused_and_its_target_is_not_written() {
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200).set_body_raw("tea", "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("link");
    let target = dir.file("target.txt");
    std::fs::write(&target, b"secret").unwrap();
    let dest = dir.file("out.json");
    std::os::unix::fs::symlink(&target, &dest).unwrap();
    for overwrite in [false, true] {
        let error = to_file(
            &service(&server),
            &[sha256(b"tea")],
            &dest,
            &FileDownloadOptions::new().with_overwrite(overwrite),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, ClientError::Io(_)), "{error:?}");
        assert_eq!(std::fs::read(&target).unwrap(), b"secret");
    }
    assert_eq!(dir.names(), ["out.json", "target.txt"]);
}

#[tokio::test]
async fn the_size_limit_applies_to_encoded_and_decoded_bytes_and_cleans_up() {
    let raw = vec![b'a'; 100_000];
    let server = MockServer::start().await;
    // Declared Content-Length above the limit.
    serve(
        &server,
        ResponseTemplate::new(200).set_body_raw(raw.clone(), "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("limit");
    let tight = Limits::default().with_max_artifact_bytes(1000);
    let client = client_with(&server, tight, false);
    let service = client.service(&server.uri(), "1.0.0").unwrap();
    let error = to_file(&service, &[], &dir.file("a"), &FileDownloadOptions::new())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::TooLarge), "{error:?}");

    // Small encoded body that expands past the limit while streaming.
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200)
            .insert_header("content-encoding", "gzip")
            .set_body_raw(gzip(&raw), "application/octet-stream"),
    )
    .await;
    let client = client_with(
        &server,
        Limits::default().with_max_artifact_bytes(5000),
        false,
    );
    let service = client.service(&server.uri(), "1.0.0").unwrap();
    let error = to_file(&service, &[], &dir.file("b"), &FileDownloadOptions::new())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Model(_)), "{error:?}");
    assert!(dir.names().is_empty(), "left behind: {:?}", dir.names());
}

#[tokio::test]
async fn a_truncated_gzip_stream_is_not_committed() {
    let raw = vec![b'z'; 50_000];
    let full = gzip(&raw);
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200)
            .insert_header("content-encoding", "gzip")
            .set_body_raw(full[..full.len() - 6].to_vec(), "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("trunc");
    // The truncated prefix decodes to bytes that match no digest, but the
    // stream error must be reported on its own: use no declared checksum.
    let error = to_file(
        &service(&server),
        &[],
        &dir.file("a"),
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, ClientError::Model(_)), "{error:?}");
    assert!(dir.names().is_empty(), "left behind: {:?}", dir.names());
}

#[tokio::test]
async fn weak_only_checksums_are_not_committed_unless_allowed() {
    let sha1 = Checksum::new(
        ChecksumAlgorithm::Sha1,
        "75490f32c06a50051804339453d3fe2f0cd5564b",
    )
    .unwrap();
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200).set_body_raw("tea", "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("weak");
    let error = to_file(
        &service(&server),
        std::slice::from_ref(&sha1),
        &dir.file("a"),
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(
            &error,
            ClientError::Integrity {
                outcome: IntegrityOutcome::Weak,
                ..
            }
        ),
        "{error:?}"
    );
    assert!(dir.names().is_empty());
    let client = client_with(&server, Limits::default(), true);
    client
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_artifact_to_file(
            ArtifactId::parse(ID).unwrap(),
            1,
            None,
            &[sha1],
            &dir.file("a"),
            &FileDownloadOptions::new(),
        )
        .await
        .unwrap();
    assert_eq!(std::fs::read(dir.file("a")).unwrap(), b"tea");
}

#[tokio::test]
async fn an_api_error_creates_no_file() {
    let server = MockServer::start().await;
    serve(&server, ResponseTemplate::new(404)).await;
    let dir = Scratch::new("404");
    let error = to_file(
        &service(&server),
        &[],
        &dir.file("a"),
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(error, ClientError::Api { status: 404, .. }),
        "{error:?}"
    );
    assert!(dir.names().is_empty());
}

#[tokio::test]
async fn a_redirect_to_another_origin_is_refused_and_creates_no_file() {
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    serve(
        &first,
        ResponseTemplate::new(302).insert_header("location", format!("{}/x", second.uri())),
    )
    .await;
    Mock::given(method("GET"))
        .and(path("/x"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("tea", "application/octet-stream"))
        .expect(0)
        .mount(&second)
        .await;
    let dir = Scratch::new("redir");
    let error = to_file(
        &service(&first),
        &[],
        &dir.file("a"),
        &FileDownloadOptions::new(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, ClientError::Policy(_)), "{error:?}");
    assert!(dir.names().is_empty());
}

#[tokio::test]
async fn the_writer_api_reports_the_same_integrity_and_flags_a_mismatch_after_writing() {
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200).set_body_raw("tea", "application/octet-stream"),
    )
    .await;
    let artifact = external_artifact(&server, &sha256(b"other"));
    let mut sink: Vec<u8> = Vec::new();
    let error = service(&server)
        .download_format_to_writer(&artifact, "application/octet-stream", &mut sink)
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Integrity { .. }), "{error:?}");
    // Documented: the writer saw the unverified bytes.
    assert_eq!(sink, b"tea");

    let good = external_artifact(&server, &sha256(b"tea"));
    let mut sink: Vec<u8> = Vec::new();
    let result = service(&server)
        .download_format_to_writer(&good, "application/octet-stream", &mut sink)
        .await
        .unwrap();
    assert_eq!(sink, b"tea");
    assert_eq!(result.bytes_written, 3);
    assert_eq!(result.integrity.outcome, IntegrityOutcome::Verified);
}

#[tokio::test]
async fn an_external_url_gets_no_tea_credentials() {
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200).set_body_raw("tea", "application/octet-stream"),
    )
    .await;
    let artifact = external_artifact(&server, &sha256(b"tea"));
    let client = Client::builder()
        .destination_policy(allow(&server))
        .api_key(libtea_client::ApiKey::new("id", "secret"))
        .build()
        .unwrap();
    let mut sink: Vec<u8> = Vec::new();
    client
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_format_to_writer(&artifact, "application/octet-stream", &mut sink)
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert!(!requests.is_empty());
    assert!(
        requests
            .iter()
            .all(|request| !request.headers.contains_key("authorization")),
        "an external artifact URL must not receive TEA credentials"
    );
}

#[tokio::test]
async fn a_cancelled_download_removes_its_temporary_file() {
    let server = MockServer::start().await;
    serve(
        &server,
        ResponseTemplate::new(200)
            .set_delay(Duration::from_secs(5))
            .set_body_raw("tea", "application/octet-stream"),
    )
    .await;
    let dir = Scratch::new("cancel");
    let service = service(&server);
    let dest = dir.file("a");
    let options = FileDownloadOptions::new();
    let pending = to_file(&service, &[], &dest, &options);
    let outcome = tokio::time::timeout(Duration::from_millis(300), pending).await;
    assert!(outcome.is_err(), "the download should still be running");
    assert!(dir.names().is_empty(), "left behind: {:?}", dir.names());
}
