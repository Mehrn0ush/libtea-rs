//! Live checks against a local Oolong process.
//!
//! These tests are ignored by `cargo test`. They are not mock coverage and
//! they do not certify TEA 1.0.0. Run them only when Oolong is already
//! listening on `127.0.0.1:3005`:
//!
//! ```text
//! cargo test -p libtea-client --test oolong_idtype_live -- --ignored --nocapture
//! ```
//!
//! The client is the SDK. A loopback proxy records the request line the SDK
//! sends, then forwards that connection to Oolong. Production destination
//! policy is unchanged; only this test client allows its proxy origin.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use libtea_client::{Client, ClientError};
use libtea_core::enums::{ComponentSort, IdentifierType, ProductSort, ReleaseSort};
use libtea_core::{Identifier, ListParams};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

const UPSTREAM: &str = "127.0.0.1:3005";

struct Transcript {
    lines: Mutex<Vec<String>>,
}

impl Transcript {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            lines: Mutex::new(Vec::new()),
        })
    }

    fn snapshot(&self) -> Vec<String> {
        self.lines.lock().expect("transcript").clone()
    }
}

struct Live {
    base: String,
    transcript: Arc<Transcript>,
}

async fn live() -> Live {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("proxy bind");
    let port = listener.local_addr().expect("proxy addr").port();
    let transcript = Transcript::new();
    let recorded = Arc::clone(&transcript);
    tokio::spawn(async move {
        loop {
            let Ok((inbound, _)) = listener.accept().await else {
                break;
            };
            let recorded = Arc::clone(&recorded);
            tokio::spawn(async move {
                if let Err(error) = forward(inbound, &recorded).await {
                    if error.kind() != std::io::ErrorKind::TimedOut
                        && error.kind() != std::io::ErrorKind::UnexpectedEof
                    {
                        eprintln!("live proxy: {error}");
                    }
                }
            });
        }
    });
    Live {
        base: format!("http://127.0.0.1:{port}"),
        transcript,
    }
}

async fn forward(mut client: TcpStream, transcript: &Transcript) -> std::io::Result<()> {
    let mut upstream = TcpStream::connect(UPSTREAM).await?;
    let mut pending = Vec::new();
    loop {
        let header_end = match read_headers(&mut client, &mut pending).await {
            Ok(end) => end,
            Err(error)
                if pending.is_empty()
                    && matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::UnexpectedEof
                    ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let headers = String::from_utf8_lossy(&pending[..header_end]).into_owned();
        let request_line = headers.lines().next().unwrap_or("").to_owned();
        transcript
            .lines
            .lock()
            .expect("transcript")
            .push(request_line);
        let total = header_end + content_length(&headers);
        read_exact_at_least(&mut client, &mut pending, total).await?;
        upstream.write_all(&pending[..total]).await?;
        upstream.flush().await?;
        pending.drain(..total);

        let mut response = Vec::new();
        let response_end = read_headers(&mut upstream, &mut response).await?;
        let response_headers = String::from_utf8_lossy(&response[..response_end]).into_owned();
        let response_total = response_end + content_length(&response_headers);
        read_exact_at_least(&mut upstream, &mut response, response_total).await?;
        client.write_all(&response[..response_total]).await?;
        client.flush().await?;
        if closes(&headers) || closes(&response_headers) {
            return Ok(());
        }
    }
}

async fn read_headers(stream: &mut TcpStream, buf: &mut Vec<u8>) -> std::io::Result<usize> {
    loop {
        if let Some(end) = header_end(buf) {
            return Ok(end);
        }
        let mut tmp = [0_u8; 4096];
        let read = tokio::time::timeout(Duration::from_secs(3), stream.read(&mut tmp))
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "header timeout"))??;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "eof before headers",
            ));
        }
        buf.extend_from_slice(&tmp[..read]);
    }
}

async fn read_exact_at_least(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
    total: usize,
) -> std::io::Result<()> {
    while buf.len() < total {
        let mut tmp = [0_u8; 4096];
        let read = tokio::time::timeout(Duration::from_secs(3), stream.read(&mut tmp))
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "body timeout"))??;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "eof before body",
            ));
        }
        buf.extend_from_slice(&tmp[..read]);
    }
    Ok(())
}

fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn closes(headers: &str) -> bool {
    headers.lines().any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.eq_ignore_ascii_case("connection") && value.trim().eq_ignore_ascii_case("close")
    })
}

fn client(base: &str) -> libtea_client::Service {
    let origin = Url::parse(base).expect("proxy origin");
    Client::builder()
        .destination_policy(libtea_core::DestinationPolicy::allow_origin(&origin).expect("origin"))
        .build()
        .expect("client")
        .service(base, "1.0.0")
        .expect("service")
}

fn params<S>() -> ListParams<S> {
    ListParams {
        page_size: None,
        page_token: None,
        sort_field: None,
        sort_order: None,
        id_type: None,
        id_value: None,
    }
}

fn purl(identifiers: &[Identifier]) -> bool {
    identifiers
        .iter()
        .any(|identifier| identifier.id_type == IdentifierType::Purl)
}

fn show(label: &str, requests: &[String]) {
    eprintln!("{label}");
    for line in requests {
        eprintln!("  {line}");
    }
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn type_only_product_page_returns_alpha() {
    let live = live().await;
    let service = client(&live.base);
    let mut query = params::<ProductSort>();
    query.page_size = Some(1);
    query.id_type = Some(IdentifierType::Purl);
    let page = service
        .query_products(&query)
        .await
        .expect("type-only page");
    let requests = live.transcript.snapshot();
    show("test 1 requests:", &requests);
    eprintln!(
        "test 1 response: names={:?} has_next={} token_present={}",
        page.results
            .iter()
            .map(|product| product.name.as_str())
            .collect::<Vec<_>>(),
        page.has_next,
        page.next_page_token.is_some()
    );
    assert_eq!(requests.len(), 1, "one SDK request: {requests:?}");
    assert!(
        requests[0].contains("GET /v1.0.0/products?"),
        "path: {}",
        requests[0]
    );
    assert!(
        requests[0].contains("idType=PURL"),
        "idType: {}",
        requests[0]
    );
    assert!(
        requests[0].contains("pageSize=1"),
        "pageSize: {}",
        requests[0]
    );
    assert!(
        !requests[0].contains("idValue="),
        "no idValue: {}",
        requests[0]
    );
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].name, "Alpha Fixture");
    assert!(purl(&page.results[0].identifiers));
    assert!(page.has_next);
    assert!(page
        .next_page_token
        .as_ref()
        .is_some_and(|token| !token.is_empty()));
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn type_only_continuation_returns_beta() {
    let live = live().await;
    let service = client(&live.base);
    let mut query = params::<ProductSort>();
    query.page_size = Some(1);
    query.id_type = Some(IdentifierType::Purl);
    let first = service.query_products(&query).await.expect("first page");
    query.page_token = first.next_page_token.clone();
    let second = service.query_products(&query).await.expect("second page");
    let requests = live.transcript.snapshot();
    show("test 2 requests:", &requests);
    eprintln!(
        "test 2 response: name={:?} has_next={}",
        second.results.first().map(|product| product.name.as_str()),
        second.has_next
    );
    assert_eq!(requests.len(), 2, "two SDK requests: {requests:?}");
    assert!(
        requests[1].contains("idType=PURL"),
        "filter kept: {}",
        requests[1]
    );
    assert!(
        requests[1].contains("pageSize=1"),
        "page size kept: {}",
        requests[1]
    );
    assert!(
        requests[1].contains("pageToken="),
        "token sent: {}",
        requests[1]
    );
    assert!(
        !requests[1].contains("idValue="),
        "no idValue: {}",
        requests[1]
    );
    assert_eq!(second.results.len(), 1);
    assert_eq!(second.results[0].name, "Beta Fixture");
    assert!(purl(&second.results[0].identifiers));
    assert!(!second.has_next);
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn purl_and_value_return_only_beta() {
    let live = live().await;
    let service = client(&live.base);
    let mut query = params::<ProductSort>();
    query.id_type = Some(IdentifierType::Purl);
    query.id_value = Some("pkg:generic/oolong-fixture/beta".to_owned());
    let page = service.query_products(&query).await.expect("paired filter");
    let requests = live.transcript.snapshot();
    show("test 3 requests:", &requests);
    eprintln!(
        "test 3 response: {:?}",
        page.results
            .iter()
            .map(|product| {
                (
                    product.name.as_str(),
                    product
                        .identifiers
                        .iter()
                        .map(|identifier| {
                            format!("{}={}", identifier.id_type.as_str(), identifier.id_value)
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("idType=PURL"));
    assert!(requests[0].contains("idValue="));
    assert!(
        requests[0].contains("pkg%3Ageneric%2Foolong-fixture%2Fbeta")
            || requests[0].contains("pkg:generic/oolong-fixture/beta"),
        "encoded value: {}",
        requests[0]
    );
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].name, "Beta Fixture");
    assert!(page.results[0].identifiers.iter().any(|identifier| {
        identifier.id_type == IdentifierType::Purl
            && identifier.id_value == "pkg:generic/oolong-fixture/beta"
    }));
    assert!(!page.has_next);
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn value_only_filter_does_not_reach_oolong() {
    let live = live().await;
    let service = client(&live.base);
    let mut query = params::<ProductSort>();
    query.id_value = Some("pkg:generic/oolong-fixture/beta".to_owned());
    let error = service
        .query_products(&query)
        .await
        .expect_err("validation");
    let requests = live.transcript.snapshot();
    show("test 4 requests:", &requests);
    eprintln!("test 4 error: {error}");
    assert!(requests.is_empty(), "no HTTP request: {requests:?}");
    match error {
        ClientError::Model(libtea_core::ModelError::Invalid(message)) => {
            assert_eq!(message, "idValue requires idType");
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn unfiltered_product_list_is_unchanged() {
    let live = live().await;
    let service = client(&live.base);
    let page = service
        .query_products(&params::<ProductSort>())
        .await
        .expect("unfiltered products");
    let requests = live.transcript.snapshot();
    show("test 5 requests:", &requests);
    let names: Vec<_> = page
        .results
        .iter()
        .map(|product| product.name.as_str())
        .collect();
    eprintln!("test 5 response: {names:?} has_next={}", page.has_next);
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0].starts_with("GET /v1.0.0/products HTTP/1.1"),
        "no query: {}",
        requests[0]
    );
    assert!(!page.has_next);
    assert_eq!(
        names,
        [
            "Alpha Fixture",
            "Beta Fixture",
            "Delta Fixture",
            "Epsilon Fixture",
            "Gamma Fixture"
        ]
    );
    assert!(page
        .results
        .iter()
        .any(|product| !purl(&product.identifiers)));
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn type_only_filter_applies_to_each_list_endpoint() {
    let live = live().await;
    let service = client(&live.base);

    let mut products = params::<ProductSort>();
    products.id_type = Some(IdentifierType::Purl);
    let products_page = service.query_products(&products).await.expect("products");
    let product_names: Vec<_> = products_page
        .results
        .iter()
        .map(|product| product.name.clone())
        .collect();

    let mut releases = params::<ReleaseSort>();
    releases.id_type = Some(IdentifierType::Purl);
    let release_page = service
        .query_product_releases(&releases)
        .await
        .expect("product releases");
    let unfiltered_releases = service
        .query_product_releases(&params::<ReleaseSort>())
        .await
        .expect("all product releases");

    let mut components = params::<ComponentSort>();
    components.id_type = Some(IdentifierType::Purl);
    let component_page = service
        .query_components(&components)
        .await
        .expect("components");
    let unfiltered_components = service
        .query_components(&params::<ComponentSort>())
        .await
        .expect("all components");

    let mut component_releases = params::<ReleaseSort>();
    component_releases.id_type = Some(IdentifierType::Purl);
    let component_release_page = service
        .query_component_releases(&component_releases)
        .await
        .expect("component releases");
    let unfiltered_component_releases = service
        .query_component_releases(&params::<ReleaseSort>())
        .await
        .expect("all component releases");

    let requests = live.transcript.snapshot();
    show("test 6 requests:", &requests);
    eprintln!("products with PURL: {product_names:?}");
    eprintln!(
        "product releases with PURL: {:?} (unfiltered {})",
        release_page
            .results
            .iter()
            .map(|release| release.version.as_str())
            .collect::<Vec<_>>(),
        unfiltered_releases.results.len()
    );
    eprintln!(
        "components with PURL: {:?} (unfiltered {})",
        component_page
            .results
            .iter()
            .map(|component| component.name.as_str())
            .collect::<Vec<_>>(),
        unfiltered_components.results.len()
    );
    eprintln!(
        "component releases with PURL: {:?} (unfiltered {})",
        component_release_page
            .results
            .iter()
            .map(|release| {
                format!(
                    "{} {}",
                    release.component_name.as_deref().unwrap_or("?"),
                    release.version
                )
            })
            .collect::<Vec<_>>(),
        unfiltered_component_releases.results.len()
    );

    assert!(requests
        .iter()
        .any(|line| line.contains("GET /v1.0.0/products?")
            && line.contains("idType=PURL")
            && !line.contains("idValue=")));
    assert!(requests
        .iter()
        .any(|line| line.contains("GET /v1.0.0/productReleases?") && line.contains("idType=PURL")));
    assert!(requests
        .iter()
        .any(|line| line.contains("GET /v1.0.0/components?") && line.contains("idType=PURL")));
    assert!(requests.iter().any(
        |line| line.contains("GET /v1.0.0/componentReleases?") && line.contains("idType=PURL")
    ));

    assert_eq!(product_names, ["Alpha Fixture", "Beta Fixture"]);
    assert!(products_page
        .results
        .iter()
        .all(|product| purl(&product.identifiers)));

    assert_eq!(unfiltered_releases.results.len(), 3);
    assert_eq!(release_page.results.len(), 1);
    assert_eq!(release_page.results[0].version, "1.0.0");
    assert!(purl(
        release_page.results[0]
            .identifiers
            .as_deref()
            .unwrap_or(&[])
    ));
    assert!(release_page.results[0]
        .identifiers
        .as_ref()
        .is_some_and(|ids| {
            ids.iter()
                .any(|id| id.id_value == "pkg:generic/oolong-fixture/widget@1.0.0")
        }));

    assert_eq!(unfiltered_components.results.len(), 2);
    assert_eq!(component_page.results.len(), 1);
    assert_eq!(component_page.results[0].name, "Fixture Part");
    assert!(purl(&component_page.results[0].identifiers));

    assert_eq!(unfiltered_component_releases.results.len(), 3);
    assert_eq!(component_release_page.results.len(), 1);
    assert_eq!(component_release_page.results[0].version, "1.0.0");
    assert_eq!(
        component_release_page.results[0].component_name.as_deref(),
        Some("Fixture Part")
    );
    assert!(purl(
        component_release_page.results[0]
            .identifiers
            .as_deref()
            .unwrap_or(&[])
    ));
}
