//! `docs/schemas/lock-v1.schema.json` must describe exactly the keys the
//! library writes. (There is no JSON Schema validator in the workspace; this
//! checks the parts that can drift: key names, requiredness, closed objects,
//! the version constant and the integrity names.)

use std::collections::BTreeSet;

use libtea_core::{LockFile, LockedArtifact, LockedChecksum, LockedIntegrity};

fn schema() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/schemas/lock-v1.schema.json"
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn keys(value: &serde_json::Value) -> BTreeSet<String> {
    value.as_object().unwrap().keys().cloned().collect()
}

fn required(value: &serde_json::Value) -> BTreeSet<String> {
    value["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn the_schema_matches_what_the_library_writes() {
    let mut lock = LockFile::new(
        "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
        "https://tea.example.com/",
        "1.0.0",
        "962a0228af1bfcc1232ac3f79239317ff33fdb04",
        "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
        1,
    );
    lock.artifacts.push(LockedArtifact::new(
        "11111111-1111-4111-8111-111111111111",
        1,
        "BOM",
        "text/plain",
        3,
        vec![LockedChecksum::new(
            "SHA-256",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        )],
        LockedIntegrity::Verified,
        "2026-11-08T00:00:00Z",
    ));
    let written: serde_json::Value = serde_json::from_str(&lock.to_json().unwrap()).unwrap();
    let schema = schema();

    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(keys(&schema["properties"]), keys(&written));
    assert_eq!(required(&schema), keys(&written));
    assert_eq!(schema["properties"]["schemaVersion"]["const"], 1);

    let entry = &schema["properties"]["artifacts"]["items"];
    assert_eq!(entry["additionalProperties"], false);
    assert_eq!(keys(&entry["properties"]), keys(&written["artifacts"][0]));
    assert_eq!(required(entry), keys(&written["artifacts"][0]));

    let checksum = &entry["properties"]["checksums"]["items"];
    assert_eq!(checksum["additionalProperties"], false);
    assert_eq!(
        keys(&checksum["properties"]),
        keys(&written["artifacts"][0]["checksums"][0])
    );
    assert_eq!(
        required(checksum),
        keys(&written["artifacts"][0]["checksums"][0])
    );

    let names: BTreeSet<String> = entry["properties"]["integrity"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    let expected: BTreeSet<String> = [
        LockedIntegrity::Verified,
        LockedIntegrity::VerifiedWeakOnly,
        LockedIntegrity::NotDeclared,
    ]
    .iter()
    .map(|i| i.as_str().to_owned())
    .collect();
    assert_eq!(names, expected);
}
