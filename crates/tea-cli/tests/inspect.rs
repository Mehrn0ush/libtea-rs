//! `libtea inspect` against a mock TEA server (`docs/cli-json.md`).

use std::process::{Command, Output};

use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REL: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const REL2: &str = "4cc5e0f3-5d6f-4d1a-8e5a-0e9d2f4a9b11";
const PRODUCT: &str = "33333333-3333-4333-8333-333333333333";
const COMPONENT: &str = "44444444-4444-4444-8444-444444444444";
const COMPONENT_RELEASE: &str = "55555555-5555-4555-8555-555555555555";
const A1: &str = "11111111-1111-4111-8111-111111111111";
const COLLECTION: &str = "66666666-6666-4666-8666-666666666666";
const TEI: &str = "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";

fn libtea(args: &[String]) -> Output {
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

fn inspect(server: &MockServer, extra: &[&str]) -> Output {
    let mut args: Vec<String> = vec![
        "inspect".into(),
        TEI.into(),
        "--well-known-url".into(),
        format!("{}/.well-known/tea", server.uri()),
        "--allow-origin".into(),
        server.uri(),
    ];
    args.extend(extra.iter().map(|arg| (*arg).to_owned()));
    libtea(&args)
}

fn collection(version: u64) -> serde_json::Value {
    serde_json::json!({
        "uuid": COLLECTION,
        "version": version,
        "createdDate": "2024-02-01T00:00:00Z",
        "belongsTo": "PRODUCT_RELEASE",
        "updateReason": {"type": "ARTIFACT_ADDED"},
        "artifacts": [{
            "uuid": A1,
            "version": 2,
            "name": "SBOM",
            "type": "BOM",
            "createdDate": "2024-02-01T00:00:00Z",
            "formats": [
                {"mediaType": "application/vnd.cyclonedx+json",
                 "url": "https://cdn.example.com/sbom.json?sig=secret",
                 "signatureUrl": "https://cdn.example.com/sbom.json.sig",
                 "checksums": [{"algType": "SHA-256", "algValue": "a".repeat(64)}]},
                {"mediaType": "application/spdx+json"}
            ]
        }]
    })
}

async fn mount(server: &MockServer, releases: &[&str]) {
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [{"url": server.uri(), "versions": ["1.0.0"], "priority": 1}]
        })))
        .mount(server)
        .await;
    let found: Vec<_> = releases
        .iter()
        .map(|id| {
            serde_json::json!({
                "productReleaseUuid": id,
                "servers": [{"rootUrl": server.uri(), "versions": ["1.0.0"], "priority": 1}]
            })
        })
        .collect();
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(ResponseTemplate::new(200).set_body_json(found))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{REL}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": REL,
            "product": PRODUCT,
            "version": "2.1.0",
            "createdDate": "2024-01-01T00:00:00Z",
            "releaseDate": "2024-01-15T00:00:00Z",
            "components": [
                {"uuid": COMPONENT, "release": COMPONENT_RELEASE},
                {"uuid": COMPONENT}
            ],
            "latestCollection": collection(3)
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": PRODUCT,
            "name": "Acme App",
            "identifiers": [{"idType": "PURL", "idValue": "pkg:npm/acme-app"}]
        })))
        .mount(server)
        .await;
    // Inspect never downloads anything.
    Mock::given(method("GET"))
        .and(path_regex(r"/download$"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(server)
        .await;
}

#[tokio::test]
async fn inspect_json_summarizes_the_release_without_downloading() {
    let server = MockServer::start().await;
    mount(&server, &[REL]).await;
    let output = inspect(&server, &["--json"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let value = json(&output.stdout);
    let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "collection",
            "product",
            "product_release",
            "schema_version",
            "server",
            "tei"
        ]
    );
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["tei"], TEI);
    assert_eq!(value["server"]["api_version"], "1.0.0");
    assert_eq!(value["product"]["name"], "Acme App");
    assert_eq!(
        value["product"]["identifiers"],
        serde_json::json!([{"type": "PURL", "value": "pkg:npm/acme-app"}])
    );
    let release = &value["product_release"];
    assert_eq!(release["uuid"], REL);
    assert_eq!(release["version"], "2.1.0");
    assert_eq!(release["release_date"], "2024-01-15T00:00:00Z");
    assert_eq!(
        release["components"],
        serde_json::json!([
            {"uuid": COMPONENT, "release": COMPONENT_RELEASE},
            {"uuid": COMPONENT, "release": null}
        ])
    );
    let collection = &value["collection"];
    assert_eq!(collection["version"], 3);
    assert_eq!(collection["update_reason"], "ARTIFACT_ADDED");
    let artifact = &collection["artifacts"][0];
    assert_eq!(artifact["uuid"], A1);
    assert_eq!(artifact["type"], "BOM");
    assert_eq!(artifact["version"], 2);
    assert_eq!(
        artifact["formats"],
        serde_json::json!([
            {"media_type": "application/vnd.cyclonedx+json", "checksum_algorithms": ["SHA-256"],
             "has_url": true, "has_signature": true},
            {"media_type": "application/spdx+json", "checksum_algorithms": [],
             "has_url": false, "has_signature": false}
        ])
    );
    // Artifact URLs can carry signed queries; the summary does not print them.
    assert!(!text(&output.stdout).contains("sig=secret"));
}

#[tokio::test]
async fn inspect_text_has_one_line_per_object() {
    let server = MockServer::start().await;
    mount(&server, &[REL]).await;
    let output = inspect(&server, &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    for line in [
        // Redacted and normalized the way `sync` records it in a lock.
        format!("server\t{}/\t1.0.0", server.uri()),
        format!("product\t{PRODUCT}\tAcme App"),
        "identifier\tPURL\tpkg:npm/acme-app".to_owned(),
        format!(
            "product_release\t{REL}\t2.1.0\tcreated 2024-01-01T00:00:00Z\treleased 2024-01-15T00:00:00Z"
        ),
        format!("component\t{COMPONENT}\t{COMPONENT_RELEASE}"),
        format!("component\t{COMPONENT}\t-"),
        format!(
            "collection\t{COLLECTION}\tversion 3\tcreated 2024-02-01T00:00:00Z\treason ARTIFACT_ADDED\tartifacts 1"
        ),
        format!("artifact\t{A1}\tBOM\tversion 2\tSBOM"),
        "format\tapplication/vnd.cyclonedx+json\tchecksums SHA-256\tsignature published".to_owned(),
        "format\tapplication/spdx+json\tchecksums -\tsignature -".to_owned(),
    ] {
        assert!(stdout.contains(&format!("{line}\n")), "missing {line:?} in\n{stdout}");
    }
    assert!(!stdout.contains("sig=secret"));
}

#[tokio::test]
async fn a_collection_version_is_fetched_when_asked_for() {
    let server = MockServer::start().await;
    mount(&server, &[REL]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{REL}/collection/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(collection(1)))
        .expect(1)
        .mount(&server)
        .await;
    let output = inspect(&server, &["--collection-version", "1", "--json"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(json(&output.stdout)["collection"]["version"], 1);
}

#[tokio::test]
async fn several_releases_need_a_choice_and_a_wrong_choice_is_not_found() {
    let server = MockServer::start().await;
    mount(&server, &[REL, REL2]).await;
    let output = inspect(&server, &["--json"]);
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
    assert!(text(&output.stderr).contains("--product-release"));

    let output = inspect(&server, &["--product-release", REL, "--json"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(json(&output.stdout)["product_release"]["uuid"], REL);

    let other = "77777777-7777-4777-8777-777777777777";
    let output = inspect(&server, &["--product-release", other, "--json"]);
    assert_eq!(output.status.code(), Some(5), "{}", text(&output.stderr));
}

#[test]
fn a_malformed_tei_is_a_usage_error() {
    let output = libtea(&["inspect".into(), "not-a-tei".into(), "--json".into()]);
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
    assert_eq!(json(&output.stderr)["error"]["kind"], "usage");
}
