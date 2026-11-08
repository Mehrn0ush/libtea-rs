//! Server-supplied strings the CLI prints must not reach the terminal with
//! control characters: ESC, BEL, CR, DEL, and the C1 range (including the
//! single-character CSI and OSC introducers).

use std::process::{Command, Output};

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
/// CSI colour, OSC title with BEL terminator, OSC with ST, single-byte CSI
/// (U+009B) and OSC (U+009D), carriage return, DEL, and a bidi override.
const HOSTILE: &str =
    "\u{1b}[31mred\u{1b}]0;pwned\u{7}\u{1b}]8;;http://evil\u{1b}\\x\u{9b}2J\u{9d}0;t\r\u{7f}\u{202e}rtl";

fn run(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(args)
        .env("RUST_LOG", "off")
        .output()
        .unwrap()
}

fn base_args(server: &MockServer, command: &[&str]) -> Vec<String> {
    let mut args: Vec<String> = command.iter().map(|part| (*part).to_owned()).collect();
    args.extend(["--base".to_owned(), server.uri()]);
    args.extend(["--allow-origin".to_owned(), server.uri()]);
    args
}

/// No control character other than tab and newline, and no bidi override.
fn assert_clean(label: &str, bytes: &[u8]) {
    let text = String::from_utf8(bytes.to_vec()).expect("CLI output is UTF-8");
    for c in text.chars() {
        let bad = match c {
            '\t' | '\n' => false,
            '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' => true,
            c => c.is_control(),
        };
        assert!(
            !bad,
            "{label}: raw U+{:04X} reached the terminal in {text:?}",
            u32::from(c)
        );
    }
}

fn collections_page() -> serde_json::Value {
    serde_json::json!({
        "hasNext": true,
        "nextPageToken": HOSTILE,
        "results": []
    })
}

#[tokio::test]
async fn text_output_of_a_server_token_is_escaped() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}/collections")))
        .respond_with(ResponseTemplate::new(200).set_body_json(collections_page()))
        .mount(&server)
        .await;
    let mut args = base_args(&server, &["collections", "--uuid", ID]);
    args.retain(|a| a != "--json");
    let output = run(&args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_clean("stdout", &output.stdout);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("\\u001b[31mred"), "{text}");
    assert!(text.contains("\\u009b2J"), "{text}");
    assert!(text.contains("\\u009d0;t"), "{text}");
    assert!(text.contains("\\u202ertl"), "{text}");
}

#[tokio::test]
async fn json_output_keeps_c1_controls_escaped_and_valid() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}/collections")))
        .respond_with(ResponseTemplate::new(200).set_body_json(collections_page()))
        .mount(&server)
        .await;
    let mut args = base_args(&server, &["collections", "--uuid", ID]);
    args.push("--json".to_owned());
    let output = run(&args);
    assert!(output.status.success());
    assert_clean("stdout", &output.stdout);
    // The raw output carries escapes (serde's own for C0, ours for C1 and bidi) ...
    let raw = String::from_utf8_lossy(&output.stdout);
    assert!(raw.contains("\\u009b2J"), "{raw}");
    assert!(raw.contains("\\u202ertl"), "{raw}");
    // ... and is still valid JSON that decodes to the server's original string.
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["nextPageToken"].as_str(), Some(HOSTILE));
}

#[tokio::test]
async fn token_error_text_is_escaped_on_stderr() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": HOSTILE
        })))
        .mount(&server)
        .await;
    let mut args = base_args(&server, &["product-release", "--uuid", ID]);
    args.extend([
        "--api-key-id".to_owned(),
        "id".to_owned(),
        "--api-key-secret".to_owned(),
        "secret".to_owned(),
    ]);
    let output = run(&args);
    assert!(!output.status.success());
    assert_clean("stderr", &output.stderr);
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("\\u001b[31mred"), "{text}");
    assert!(!text.contains("secret"));
}

#[tokio::test]
async fn api_error_code_is_escaped_on_stderr() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": HOSTILE,
            "message": HOSTILE
        })))
        .mount(&server)
        .await;
    let output = run(&base_args(&server, &["product-release", "--uuid", ID]));
    assert!(!output.status.success());
    assert_clean("stderr", &output.stderr);
    assert!(String::from_utf8_lossy(&output.stderr).contains("\\u001b"));
}
