//! `discover --purl` and `cle` (`docs/cli-json.md`, `docs/cli-exit-codes.md`).

use std::process::{Command, Output};

use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REL: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
/// Not canonical: uppercase type and qualifier key, qualifiers out of order.
const PURL: &str = "pkg:NPM/left-pad@1.3.0?B=2&a=1";

fn libtea(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(args)
        .env("RUST_LOG", "off")
        .env_remove("TEA_API_KEY_ID")
        .env_remove("TEA_API_KEY_SECRET")
        .env_remove("LIBTEA_CACHE_DIR")
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap_or_else(|err| panic!("not JSON ({err}): {}", text(bytes)))
}

fn discover_purl(server: &MockServer, purl: &str, extra: &[&str]) -> Output {
    let uri = server.uri();
    let mut args = vec![
        "discover",
        "--purl",
        purl,
        "--base",
        &uri,
        "--allow-origin",
        &uri,
    ];
    args.extend_from_slice(extra);
    libtea(&args)
}

#[tokio::test]
async fn discover_purl_sends_the_purl_as_given_and_resolves_the_release() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .and(query_param("purl", PURL))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                "productReleaseUuid": REL,
                "servers": [{"rootUrl": server.uri(), "versions": ["1.0.0"], "priority": 1}]
            }])),
        )
        .expect(2)
        .mount(&server)
        .await;

    let output = discover_purl(&server, PURL, &["--json"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let rows = json(&output.stdout);
    assert_eq!(
        rows,
        serde_json::json!([{
            "product_release": REL,
            "root_url": server.uri(),
            "api_version": "1.0.0",
        }])
    );

    let output = discover_purl(&server, PURL, &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        format!("{REL}\t{}\t1.0.0\n", server.uri())
    );
}

#[tokio::test]
async fn an_invalid_purl_is_a_usage_error_and_nothing_is_sent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    for bad in ["npm/left-pad", "pkg:", "pkg:npm/", "pkg:9npm/left-pad"] {
        let output = discover_purl(&server, bad, &["--json"]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{bad}: {}",
            text(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert_eq!(json(&output.stderr)["error"]["kind"], "usage");
    }
}

#[test]
fn purl_and_tei_options_are_checked_by_the_command_line() {
    let cases: &[&[&str]] = &[
        &["discover"],
        &["discover", "--purl", PURL],
        &["discover", "--base", "https://tea.example.com"],
        &[
            "discover",
            "urn:tei:uuid:tea.example.com:2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
            "--purl",
            PURL,
            "--base",
            "https://tea.example.com",
        ],
        &[
            "discover",
            "--purl",
            PURL,
            "--base",
            "https://tea.example.com",
            "--well-known-url",
            "https://tea.example.com/.well-known/tea",
        ],
        &[
            "discover",
            "--purl",
            PURL,
            "--base",
            "https://tea.example.com",
            "--allow-discovered-credentials",
        ],
    ];
    for args in cases {
        let output = libtea(args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            text(&output.stderr)
        );
    }
}

#[tokio::test]
async fn an_unresolved_purl_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_raw(r#"{"error":"OBJECT_UNKNOWN"}"#, "application/json"),
        )
        .mount(&server)
        .await;
    let output = discover_purl(&server, PURL, &["--json"]);
    assert_eq!(output.status.code(), Some(5), "{}", text(&output.stderr));
    assert_eq!(json(&output.stderr)["error"]["kind"], "not_found");
}

/// Events for the tests below: EOL for `<2.0.0` (SemVer), end of support for a
/// PyPI range (not evaluated), released `1.4.0`, and a withdrawn EOL.
fn cle_body() -> serde_json::Value {
    serde_json::json!({
        "events": [
            {"id": 5, "type": "withdrawn", "effective": "2025-03-01T00:00:00Z",
             "published": "2025-03-01T00:00:00Z", "eventId": 4},
            {"id": 4, "type": "endOfLife", "effective": "2025-02-01T00:00:00Z",
             "published": "2025-02-01T00:00:00Z", "versions": [{"range": "vers:semver/<1.5.0"}]},
            {"id": 3, "type": "endOfLife", "effective": "2026-01-01T00:00:00Z",
             "published": "2025-01-01T00:00:00Z", "versions": [{"range": "vers:semver/<2.0.0"}]},
            {"id": 2, "type": "endOfSupport", "effective": "2025-06-01T00:00:00Z",
             "published": "2025-01-01T00:00:00Z", "versions": [{"range": "vers:pypi/<2.0"}]},
            {"id": 1, "type": "released", "effective": "2024-01-01T00:00:00Z",
             "published": "2024-01-01T00:00:00Z", "version": "1.4.0"}
        ]
    })
}

async fn serve_cle(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{REL}/cle")))
        .respond_with(response)
        .mount(server)
        .await;
}

fn cle(server: &MockServer, extra: &[&str]) -> Output {
    let uri = server.uri();
    let mut args = vec![
        "cle",
        "--base",
        &uri,
        "--product-release",
        REL,
        "--allow-origin",
        &uri,
    ];
    args.extend_from_slice(extra);
    libtea(&args)
}

#[tokio::test]
async fn cle_json_with_a_version_carries_the_status_and_what_was_not_evaluated() {
    let server = MockServer::start().await;
    serve_cle(
        &server,
        ResponseTemplate::new(200).set_body_json(cle_body()),
    )
    .await;
    let output = cle(&server, &["--version", "1.4.0", "--json"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let value = json(&output.stdout);
    let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "cle",
            "implemented",
            "message",
            "schema_version",
            "status",
            "subject",
            "version"
        ]
    );
    assert_eq!(value["schema_version"], 1);
    assert_eq!(
        value["subject"],
        serde_json::json!({"type": "product_release", "uuid": REL})
    );
    assert_eq!(value["implemented"], true);
    assert_eq!(value["version"], "1.4.0");
    assert_eq!(value["cle"]["events"].as_array().unwrap().len(), 5);
    let status = &value["status"];
    assert_eq!(status["released"], "2024-01-01T00:00:00Z");
    // Event 4 is withdrawn; event 3 applies.
    assert_eq!(status["end_of_life"], "2026-01-01T00:00:00Z");
    assert_eq!(status["withdrawn_events"], serde_json::json!([4]));
    // Not "no end of support": the PyPI range was not evaluated.
    assert_eq!(status["end_of_support"], serde_json::Value::Null);
    assert_eq!(
        status["unevaluated"],
        serde_json::json!([{"event_id": 2, "reason": "unsupported_scheme", "detail": "pypi"}])
    );
}

#[tokio::test]
async fn cle_text_with_a_version_warns_about_events_not_evaluated() {
    let server = MockServer::start().await;
    serve_cle(
        &server,
        ResponseTemplate::new(200).set_body_json(cle_body()),
    )
    .await;
    let output = cle(&server, &["--version", "1.4.0"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(stdout.contains("version\t1.4.0\n"), "{stdout}");
    assert!(
        stdout.contains("end_of_life\t2026-01-01T00:00:00Z\n"),
        "{stdout}"
    );
    assert!(stdout.contains("end_of_support\t-\n"), "{stdout}");
    assert!(stdout.contains("withdrawn_event\t4\n"), "{stdout}");
    assert!(
        stdout.contains("unevaluated\t2\tunsupported_scheme\tpypi\n"),
        "{stdout}"
    );
    assert!(text(&output.stderr).contains("could not be evaluated"));
}

#[tokio::test]
async fn cle_without_a_version_prints_the_events_only() {
    let server = MockServer::start().await;
    serve_cle(
        &server,
        ResponseTemplate::new(200).set_body_json(cle_body()),
    )
    .await;
    let output = cle(&server, &["--json"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let value = json(&output.stdout);
    assert!(value.get("version").is_none());
    assert!(value.get("status").is_none());
    assert_eq!(value["cle"]["events"][0]["id"], 5);
}

#[tokio::test]
async fn cle_not_implemented_is_reported_and_never_looks_like_a_status() {
    let server = MockServer::start().await;
    serve_cle(
        &server,
        ResponseTemplate::new(404).set_body_raw(
            r#"{"error":"NOT_IMPLEMENTED","message":"no lifecycle data"}"#,
            "application/json",
        ),
    )
    .await;
    let output = cle(&server, &["--version", "1.4.0", "--json"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let value = json(&output.stdout);
    assert_eq!(value["implemented"], false);
    assert_eq!(value["message"], "no lifecycle data");
    assert_eq!(value["cle"], serde_json::Value::Null);
    assert_eq!(value["status"], serde_json::Value::Null);

    let output = cle(&server, &["--version", "1.4.0"]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(text(&output.stderr).contains("lifecycle not implemented by the server"));
}

#[tokio::test]
async fn cle_for_an_unknown_object_is_not_found() {
    let server = MockServer::start().await;
    serve_cle(
        &server,
        ResponseTemplate::new(404)
            .set_body_raw(r#"{"error":"OBJECT_UNKNOWN"}"#, "application/json"),
    )
    .await;
    let output = cle(&server, &["--json"]);
    assert_eq!(output.status.code(), Some(5), "{}", text(&output.stderr));
}

#[tokio::test]
async fn cle_bad_input_is_a_usage_error_before_any_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    for version in ["1.4", "v1.4.0", "latest"] {
        let output = cle(&server, &["--version", version, "--json"]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{version}: {}",
            text(&output.stderr)
        );
        assert_eq!(json(&output.stderr)["error"]["kind"], "usage");
    }
    let uri = server.uri();
    let cases: &[&[&str]] = &[
        &["cle", "--base", &uri],
        &["cle", "--base", &uri, "--product", REL, "--component", REL],
        &[
            "cle",
            "--base",
            &uri,
            "--product",
            "not-a-uuid",
            "--allow-origin",
            &uri,
        ],
    ];
    for args in cases {
        let output = libtea(args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            text(&output.stderr)
        );
    }
}

#[tokio::test]
async fn cle_uses_the_path_of_each_subject() {
    let server = MockServer::start().await;
    for (segment, flag) in [
        ("product", "--product"),
        ("productRelease", "--product-release"),
        ("component", "--component"),
        ("componentRelease", "--component-release"),
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .and(path(format!("/v1.0.0/{segment}/{REL}/cle")))
            .respond_with(ResponseTemplate::new(200).set_body_json(cle_body()))
            .expect(1)
            .mount(&server)
            .await;
        let uri = server.uri();
        let output = libtea(&[
            "cle",
            "--base",
            &uri,
            flag,
            REL,
            "--allow-origin",
            &uri,
            "--json",
        ]);
        assert!(output.status.success(), "{flag}: {}", text(&output.stderr));
        let expected = flag.trim_start_matches("--").replace('-', "_");
        assert_eq!(json(&output.stdout)["subject"]["type"], expected.as_str());
        server.verify().await;
    }
}
