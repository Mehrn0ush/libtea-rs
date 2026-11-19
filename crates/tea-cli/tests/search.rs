//! `search` (`docs/cli-json.md`, `docs/cli-exit-codes.md`).

use std::process::{Command, Output};

use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const ID2: &str = "4cc5e0f3-5d6f-4d1a-8e5a-0e9d2f4a9b11";
/// Not canonical: uppercase type, qualifiers out of order.
const PURL: &str = "pkg:NPM/left-pad@1.3.0?b=2&a=1";

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

fn search(server: &MockServer, what: &str, extra: &[&str]) -> Output {
    let uri = server.uri();
    let mut args = vec!["search", what, "--base", &uri, "--allow-origin", &uri];
    args.extend_from_slice(extra);
    libtea(&args)
}

fn page(results: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"hasNext": true, "nextPageToken": "next-1", "results": results})
}

#[tokio::test]
async fn products_by_purl_send_the_identifier_as_given() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("idType", "PURL"))
        .and(query_param("idValue", PURL))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(serde_json::json!([
            {"uuid": ID, "name": "Left Pad", "identifiers": [{"idType": "PURL", "idValue": PURL}]}
        ]))))
        .expect(2)
        .mount(&server)
        .await;

    let output = search(
        &server,
        "products",
        &["--id-type", "PURL", "--id-value", PURL, "--json"],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    let value = json(&output.stdout);
    assert_eq!(value["hasNext"], true);
    assert_eq!(value["nextPageToken"], "next-1");
    assert_eq!(value["results"][0]["uuid"], ID);
    assert_eq!(value["results"][0]["name"], "Left Pad");

    // The lowercase spelling of the type is accepted too.
    let output = search(
        &server,
        "products",
        &["--id-type", "purl", "--id-value", PURL],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        format!("has_next true\tnext_page_token next-1\n{ID}\tLeft Pad\n")
    );
}

#[tokio::test]
async fn an_identifier_type_alone_is_sent_without_a_value() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/components"))
        .and(query_param("idType", "COMPLIANCE_DOCUMENT"))
        .and(query_param_is_missing("idValue"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(page(serde_json::json!([
                {"uuid": ID, "name": "SOC 2 report", "identifiers": []}
            ]))),
        )
        .expect(1)
        .mount(&server)
        .await;
    let output = search(&server, "components", &["--id-type", "compliance-document"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains(&format!("{ID}\tSOC 2 report\n")));
}

#[tokio::test]
async fn releases_map_sort_and_paging_options_to_the_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/productReleases"))
        .and(query_param("sortField", "releaseDate"))
        .and(query_param("sortOrder", "desc"))
        .and(query_param("pageSize", "2"))
        .and(query_param("pageToken", "tok"))
        .and(query_param("idType", "CPE"))
        .and(query_param(
            "idValue",
            "cpe:2.3:a:acme:app:1.0:*:*:*:*:*:*:*",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(page(serde_json::json!([{
                "uuid": ID, "product": ID2, "productName": "App", "version": "1.0",
                "createdDate": "2024-01-01T00:00:00Z", "components": []
            }]))),
        )
        .expect(1)
        .mount(&server)
        .await;
    let output = search(
        &server,
        "product-releases",
        &[
            "--sort-field",
            "release-date",
            "--sort-order",
            "desc",
            "--page-size",
            "2",
            "--page-token",
            "tok",
            "--id-type",
            "CPE",
            "--id-value",
            "cpe:2.3:a:acme:app:1.0:*:*:*:*:*:*:*",
        ],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains(&format!("{ID}\t1.0\t{ID2}\tApp\n")));
}

#[tokio::test]
async fn each_kind_queries_its_own_path() {
    let server = MockServer::start().await;
    for (what, segment, result) in [
        (
            "products",
            "products",
            serde_json::json!({"uuid": ID, "name": "P", "identifiers": []}),
        ),
        (
            "product-releases",
            "productReleases",
            serde_json::json!({"uuid": ID, "product": ID2, "version": "1",
                "createdDate": "2024-01-01T00:00:00Z", "components": []}),
        ),
        (
            "components",
            "components",
            serde_json::json!({"uuid": ID, "name": "C", "identifiers": []}),
        ),
        (
            "component-releases",
            "componentReleases",
            serde_json::json!({"uuid": ID, "component": ID2, "version": "1",
                "createdDate": "2024-01-01T00:00:00Z"}),
        ),
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .and(path(format!("/v1.0.0/{segment}")))
            .and(query_param_is_missing("idType"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(page(serde_json::json!([result]))),
            )
            .expect(1)
            .mount(&server)
            .await;
        let output = search(&server, what, &["--json"]);
        assert!(output.status.success(), "{what}: {}", text(&output.stderr));
        assert_eq!(json(&output.stdout)["results"][0]["uuid"], ID, "{what}");
        server.verify().await;
    }
}

#[tokio::test]
async fn bad_input_is_a_usage_error_before_any_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    let cases: &[(&str, &[&str])] = &[
        (
            "products",
            &["--id-type", "PURL", "--id-value", "npm/left-pad"],
        ),
        ("products", &["--id-type", "TEI", "--id-value", "not-a-tei"]),
        ("products", &["--sort-field", "version"]),
        ("product-releases", &["--sort-field", "name"]),
    ];
    for (what, extra) in cases {
        let mut args = extra.to_vec();
        args.push("--json");
        let output = search(&server, what, &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{what} {extra:?}: {}",
            text(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert_eq!(
            json(&output.stderr)["error"]["kind"],
            "usage",
            "{what} {extra:?}"
        );
    }
    // Rejected by the command line itself.
    let clap_cases: &[(&str, &[&str])] = &[
        ("products", &["--id-value", PURL]),
        ("products", &["--id-type", "SWID"]),
        ("products", &["--sort-order", "up"]),
        ("artifacts", &[]),
    ];
    for (what, extra) in clap_cases {
        let output = search(&server, what, extra);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{what} {extra:?}: {}",
            text(&output.stderr)
        );
    }
}

#[tokio::test]
async fn a_server_error_status_keeps_its_exit_code() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let output = search(&server, "products", &["--json"]);
    assert_eq!(output.status.code(), Some(3), "{}", text(&output.stderr));
}
