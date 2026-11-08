//! Timeouts: a large artifact that keeps arriving is not cut off by the short
//! request timeout, a stalled one is, and metadata requests stay bounded.
//!
//! The server is a bare TCP listener so the pace of the body is exact.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use libtea_client::{ArtifactFetch, Client, ClientError, FileDownloadOptions, Limits, Service};
use libtea_core::{ArtifactId, DestinationPolicy, ProductReleaseId};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

const ART: &str = "11111111-1111-4111-8111-111111111111";
const REL: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const CHUNK: usize = 1000;

/// Serve `chunks` of `CHUNK` bytes, `delay` apart. After chunk `stall_after`
/// (if any) the server goes silent for `stall`. `content_type` is the type sent.
async fn trickle(
    chunks: usize,
    delay: Duration,
    stall_after: Option<usize>,
    stall: Duration,
    content_type: &'static str,
    body_byte: u8,
) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut request = Vec::new();
                let mut buf = [0u8; 1024];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    match socket.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => request.extend_from_slice(&buf[..n]),
                    }
                }
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    chunks * CHUNK
                );
                if socket.write_all(head.as_bytes()).await.is_err() {
                    return;
                }
                for i in 0..chunks {
                    if socket.write_all(&[body_byte; CHUNK]).await.is_err() {
                        return;
                    }
                    let _ = socket.flush().await;
                    if stall_after == Some(i + 1) {
                        tokio::time::sleep(stall).await;
                    } else {
                        tokio::time::sleep(delay).await;
                    }
                }
            });
        }
    });
    addr
}

fn service(addr: SocketAddr, limits: Limits) -> Service {
    let base = format!("http://{addr}");
    Client::builder()
        .limits(
            limits
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(0),
        )
        .destination_policy(DestinationPolicy::allow_origin(&Url::parse(&base).unwrap()).unwrap())
        .build()
        .unwrap()
        .service(&base, "1.0.0")
        .unwrap()
}

fn id() -> ArtifactId {
    ArtifactId::parse(ART).unwrap()
}

#[tokio::test]
async fn a_download_that_keeps_arriving_outlives_the_request_timeout() {
    // About 1 s of body against a 400 ms request timeout. Only the request
    // timeout is set here; the download timeout and idle timeout are defaults.
    let addr = trickle(
        10,
        Duration::from_millis(100),
        None,
        Duration::ZERO,
        "application/octet-stream",
        b'a',
    )
    .await;
    let svc = service(
        addr,
        Limits::default().with_request_timeout(Duration::from_millis(400)),
    );
    let started = Instant::now();
    let fetched = svc
        .download_artifact(id(), 1, None, &[], None)
        .await
        .unwrap();
    assert!(
        started.elapsed() > Duration::from_millis(700),
        "the transfer should have taken about a second"
    );
    let ArtifactFetch::Body(body) = fetched else {
        panic!("expected a body");
    };
    assert_eq!(body.bytes.len(), 10 * CHUNK);
}

#[tokio::test]
async fn a_streamed_download_outlives_the_request_timeout_too() {
    let addr = trickle(
        10,
        Duration::from_millis(100),
        None,
        Duration::ZERO,
        "application/octet-stream",
        b'a',
    )
    .await;
    let svc = service(
        addr,
        Limits::default().with_request_timeout(Duration::from_millis(400)),
    );
    let out = std::env::temp_dir().join(format!("libtea-timeouts-{}.bin", std::process::id()));
    let _ = std::fs::remove_file(&out);
    let result = svc
        .download_artifact_to_file(id(), 1, None, &[], &out, &FileDownloadOptions::new())
        .await
        .unwrap();
    assert_eq!(result.download.bytes_written, (10 * CHUNK) as u64);
    assert_eq!(std::fs::metadata(&out).unwrap().len(), (10 * CHUNK) as u64);
    let _ = std::fs::remove_file(&out);
}

#[tokio::test]
async fn a_stalled_download_fails_after_the_idle_timeout() {
    // Two chunks, then silence for three seconds; the idle timeout is 300 ms.
    let addr = trickle(
        6,
        Duration::from_millis(20),
        Some(2),
        Duration::from_secs(3),
        "application/octet-stream",
        b'a',
    )
    .await;
    let svc = service(
        addr,
        Limits::default().with_idle_timeout(Duration::from_millis(300)),
    );
    let started = Instant::now();
    let error = svc
        .download_artifact(id(), 1, None, &[], None)
        .await
        .unwrap_err();
    assert!(
        matches!(error, ClientError::Timeout | ClientError::Transport(_)),
        "{error:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn the_download_timeout_still_caps_the_whole_transfer() {
    // Steady progress, but the transfer needs about a second and may take 500 ms.
    let addr = trickle(
        10,
        Duration::from_millis(100),
        None,
        Duration::ZERO,
        "application/octet-stream",
        b'a',
    )
    .await;
    let svc = service(
        addr,
        Limits::default()
            .with_download_timeout(Duration::from_millis(500))
            .with_idle_timeout(Duration::from_secs(5)),
    );
    let error = svc
        .download_artifact(id(), 1, None, &[], None)
        .await
        .unwrap_err();
    assert!(
        matches!(error, ClientError::Timeout | ClientError::Transport(_)),
        "{error:?}"
    );
}

#[tokio::test]
async fn metadata_requests_keep_the_short_total_timeout() {
    // A JSON document that drips for about a second against a 300 ms request timeout.
    let addr = trickle(
        10,
        Duration::from_millis(100),
        None,
        Duration::ZERO,
        "application/json",
        b' ',
    )
    .await;
    let svc = service(
        addr,
        Limits::default()
            .with_request_timeout(Duration::from_millis(300))
            .with_idle_timeout(Duration::from_secs(5)),
    );
    let started = Instant::now();
    let error = svc
        .get_product_release(ProductReleaseId::parse(REL).unwrap())
        .await
        .unwrap_err();
    assert!(
        matches!(error, ClientError::Timeout | ClientError::Transport(_)),
        "{error:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(900),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn the_defaults_and_the_zero_check_are_as_documented() {
    let limits = Limits::default();
    assert_eq!(limits.request_timeout, Duration::from_secs(30));
    assert_eq!(limits.idle_timeout, Duration::from_secs(30));
    assert!(limits.download_timeout >= Duration::from_secs(10 * 60));
    assert!(limits.download_timeout > limits.request_timeout);
    for bad in [
        Limits::default().with_idle_timeout(Duration::ZERO),
        Limits::default().with_download_timeout(Duration::ZERO),
    ] {
        assert!(Client::builder().limits(bad).build().is_err());
    }
}
