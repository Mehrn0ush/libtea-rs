//! `vex/libtea.vex.cdx.json`, the CycloneDX VEX document that records whether
//! an advisory against a dependency affects libtea (SECURITY.md). The release
//! SBOM links to it, so it must stay a document a VEX consumer can use.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

fn vex() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vex/libtea.vex.cdx.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn text<'a>(value: &'a Value, what: &str) -> &'a str {
    value
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| panic!("{what} must be a non-empty string"))
}

#[test]
fn the_vex_document_is_cyclonedx_1_5() {
    let doc = vex();
    assert_eq!(doc["bomFormat"], "CycloneDX");
    assert_eq!(doc["specVersion"], "1.5");
    let serial = text(&doc["serialNumber"], "serialNumber");
    let uuid = serial.strip_prefix("urn:uuid:").expect("urn:uuid: serial");
    assert_eq!(uuid.len(), 36, "{serial}");
    assert!(doc["version"].as_u64().is_some_and(|v| v >= 1));
    text(
        &doc["metadata"]["supplier"]["name"],
        "metadata.supplier.name",
    );
}

#[test]
fn every_component_is_a_libtea_crate_release() {
    for component in vex()["components"].as_array().unwrap() {
        let name = text(&component["name"], "component name");
        let version = text(&component["version"], "component version");
        assert!(
            ["libtea-core", "libtea-client", "libtea-cli"].contains(&name),
            "{name}"
        );
        assert_eq!(component["purl"], format!("pkg:cargo/{name}@{version}"));
        assert_eq!(component["bom-ref"], format!("{name}@{version}"));
    }
}

/// Each statement names its advisory, gives a CycloneDX analysis state, and for
/// `not_affected` a justification and the reasoning; every `affects` entry
/// points at a component of the document.
#[test]
fn every_statement_is_complete_and_refers_to_a_listed_release() {
    const STATES: &[&str] = &[
        "resolved",
        "resolved_with_pedigree",
        "exploitable",
        "in_triage",
        "false_positive",
        "not_affected",
    ];
    const JUSTIFICATIONS: &[&str] = &[
        "code_not_present",
        "code_not_reachable",
        "requires_configuration",
        "requires_dependency",
        "requires_environment",
        "protected_by_compiler",
        "protected_at_runtime",
        "protected_at_perimeter",
        "protected_by_mitigating_control",
    ];
    let doc = vex();
    let refs: BTreeSet<&str> = doc["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["bom-ref"].as_str().unwrap())
        .collect();
    let mut ids = BTreeSet::new();
    let vulnerabilities = doc["vulnerabilities"].as_array().unwrap();
    assert!(!vulnerabilities.is_empty());
    for vulnerability in vulnerabilities {
        let id = text(&vulnerability["id"], "vulnerability id");
        assert!(ids.insert(id), "{id} is listed twice");
        assert!(
            text(&vulnerability["source"]["url"], "source.url").starts_with("https://"),
            "{id}"
        );
        let analysis = &vulnerability["analysis"];
        let state = text(&analysis["state"], "analysis.state");
        assert!(STATES.contains(&state), "{id}: {state}");
        if state == "not_affected" {
            let justification = text(&analysis["justification"], "analysis.justification");
            assert!(
                JUSTIFICATIONS.contains(&justification),
                "{id}: {justification}"
            );
            text(&analysis["detail"], "analysis.detail");
        }
        let affects = vulnerability["affects"].as_array().unwrap();
        assert!(!affects.is_empty(), "{id}");
        for entry in affects {
            let target = text(&entry["ref"], "affects.ref");
            assert!(refs.contains(target), "{id}: {target} is not a component");
        }
    }
}
