//! Server-supplied text in `ClientError` / `ModelError` must not carry control
//! characters: ESC, BEL, CR, DEL, the C1 range (single-character CSI and OSC),
//! or bidi overrides. They are escaped as `\uXXXX` when the error is built.
//!
//! Uses only API that existed before the escaping, so it also runs on older
//! trees (where it fails).

use std::time::Duration;

use libtea_client::{ApiKey, Client, ClientError, Limits};
use libtea_core::{DestinationPolicy, ModelError, ProductId, Tei};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const TEI: &str = "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
/// CSI colour, OSC title (BEL and ST terminated), single-byte CSI (U+009B) and
/// OSC (U+009D), CR, DEL, and a bidi override.
const HOSTILE: &str = "\u{1b}[31mred\u{1b}]0;pwned\u{7}\u{1b}]8;;http://evil\u{1b}\\x\u{9b}2J\u{9d}0;t\r\u{7f}\u{202e}rtl";

fn is_bad(c: char) -> bool {
    match c {
        '\t' | '\n' => false,
        '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' => true,
        c => c.is_control(),
    }
}

fn assert_clean(label: &str, text: &str) {
    if let Some(c) = text.chars().find(|c| is_bad(*c)) {
        panic!("{label}: raw U+{:04X} in {text:?}", u32::from(c));
    }
}

/// Display and Debug of the error must both be clean.
fn assert_error_clean(error: &dyn std::fmt::Debug, display: &str) {
    assert_clean("display", display);
    assert_clean("debug", &format!("{error:?}"));
}

fn client(server: &MockServer, key: bool) -> Client {
    let mut builder = Client::builder()
        .limits(
            Limits::default()
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(0)
                .with_request_timeout(Duration::from_secs(5)),
        )
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap(),
        )
        .well_known_fetch_url(Url::parse(&format!("{}/.well-known/tea", server.uri())).unwrap());
    if key {
        builder = builder.api_key(ApiKey::new("id", "secret"));
    }
    builder.build().unwrap()
}

#[tokio::test]
async fn api_error_code_and_message_are_escaped() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}")))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": HOSTILE,
            "message": HOSTILE
        })))
        .mount(&server)
        .await;
    let error = client(&server, false)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(ID).unwrap())
        .await
        .unwrap_err();
    assert_error_clean(&error, &error.to_string());
    let ClientError::Api {
        tea_error, message, ..
    } = &error
    else {
        panic!("{error:?}");
    };
    for field in [tea_error.as_deref().unwrap(), message.as_deref().unwrap()] {
        assert_clean("field", field);
        assert!(field.contains("\\u001b[31mred"), "{field}");
        assert!(
            field.contains("\\u009b2J") && field.contains("\\u009d0;t"),
            "{field}"
        );
        assert!(field.contains("\\u202ertl"), "{field}");
    }
}

#[tokio::test]
async fn ordinary_codes_and_messages_are_unchanged() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}")))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "OBJECT_UNKNOWN",
            "message": "Tab\there, newline\nhere, ünïcode 日本 and a \\u001b text sequence"
        })))
        .mount(&server)
        .await;
    let error = client(&server, false)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(ID).unwrap())
        .await
        .unwrap_err();
    match error {
        ClientError::Api {
            tea_error, message, ..
        } => {
            assert_eq!(tea_error.as_deref(), Some("OBJECT_UNKNOWN"));
            assert_eq!(
                message.as_deref(),
                Some("Tab\there, newline\nhere, ünïcode 日本 and a \\u001b text sequence")
            );
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn token_error_text_is_escaped() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}")))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(serde_json::json!({ "error": HOSTILE })),
        )
        .mount(&server)
        .await;
    let error = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(ID).unwrap())
        .await
        .unwrap_err();
    assert_error_clean(&error, &error.to_string());
    let ClientError::Token { error: text, .. } = &error else {
        panic!("{error:?}");
    };
    assert!(text.contains("\\u001b[31mred"), "{text}");
}

#[tokio::test]
async fn token_endpoint_404_code_is_escaped() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}")))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({ "error": HOSTILE })),
        )
        .mount(&server)
        .await;
    let error = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(ID).unwrap())
        .await
        .unwrap_err();
    assert_error_clean(&error, &error.to_string());
}

#[tokio::test]
async fn unresolved_identifier_text_is_escaped_for_both_discovery_paths() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [{"url": server.uri(), "versions": ["1.0.0"]}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": HOSTILE,
            "message": HOSTILE
        })))
        .mount(&server)
        .await;
    let c = client(&server, false);
    let by_client = c.discover(&Tei::parse(TEI).unwrap()).await.unwrap_err();
    assert_error_clean(&by_client, &by_client.to_string());
    assert!(
        matches!(by_client, ClientError::Unresolved { .. }),
        "{by_client:?}"
    );
    let by_service = c
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .discover_identifier(Some(TEI), None)
        .await
        .unwrap_err();
    assert_error_clean(&by_service, &by_service.to_string());
    let ClientError::Unresolved { error, message, .. } = &by_service else {
        panic!("{by_service:?}");
    };
    assert!(error.contains("\\u001b"), "{error}");
    assert!(message.as_deref().unwrap().contains("\\u009b2J"));
}

#[tokio::test]
async fn model_error_quoting_a_server_value_is_escaped() {
    // The advertised version is quoted in the "not SemVer" message.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [{"url": server.uri(), "versions": [HOSTILE]}]
        })))
        .mount(&server)
        .await;
    let error = client(&server, false)
        .discover(&Tei::parse(TEI).unwrap())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Model(_)), "{error:?}");
    assert_error_clean(&error, &error.to_string());
    assert!(error.to_string().contains("\\u001b[31mred"), "{error}");
}

#[test]
fn model_error_display_escapes_control_characters() {
    for error in [
        ModelError::Invalid(HOSTILE.to_owned()),
        ModelError::Json(HOSTILE.to_owned()),
    ] {
        assert_clean("display", &error.to_string());
        assert_clean("debug", &format!("{error:?}"));
    }
    // Ordinary messages are untouched.
    assert_eq!(
        ModelError::Invalid("pageSize must be from 1 to 100".into()).to_string(),
        "pageSize must be from 1 to 100"
    );
}

#[tokio::test]
async fn json_error_text_is_clean() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            format!("{{\"uuid\": {}}}", serde_json::json!(HOSTILE)),
            "application/json",
        ))
        .mount(&server)
        .await;
    let error = client(&server, false)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(ID).unwrap())
        .await
        .unwrap_err();
    assert_error_clean(&error, &error.to_string());
}
