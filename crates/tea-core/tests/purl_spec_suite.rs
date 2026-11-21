//! The Package URL test suite (`spec/purl/`, purl-spec v1.1.1) against
//! `libtea_core::purl`.
//!
//! `parse` cases compare the components of the canonical form (the suite
//! expects normalized components, while `Purl`'s accessors return what was
//! written), `validate` cases compare the canonical string, and
//! `expected_failure` parse cases must be rejected by `Purl::parse` or
//! `Purl::check_type`. `build` cases
//! construct a PURL from components, which this module does not offer, so they
//! are skipped.
//!
//! The module applies the rules ECMA-427 gives for every type, `pypi`'s
//! PEP 503 names, and what the type definitions in `spec/purl/types/` give as
//! data: case-insensitive components, required and prohibited namespaces,
//! required qualifiers and permitted characters. `definitions_match_the_module`
//! checks those against every definition. Cases that need a rule given only
//! in prose, or that contradict another case, are listed in `TYPE_SPECIFIC`
//! with the reason; the test fails if one of them starts passing, so the list
//! stays honest.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use libtea_core::purl::{Purl, PurlRules, PurlTypeError};
use serde_json::Value;

fn suite_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/purl/tests")
}

fn suite_files() -> Vec<PathBuf> {
    let mut files = vec![suite_dir().join("spec/specification-test.json")];
    let mut types: Vec<_> = std::fs::read_dir(suite_dir().join("types"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    types.sort();
    files.extend(types);
    files
}

fn opt(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

/// What the parser produced, in the suite's shape.
fn components(purl: &Purl) -> Value {
    let qualifiers: BTreeMap<_, _> = purl
        .qualifiers()
        .map(|(k, v)| (k.to_owned(), Value::String(v.to_owned())))
        .collect();
    serde_json::json!({
        "type": purl.package_type(),
        "namespace": purl.namespace(),
        "name": purl.name(),
        "version": purl.version(),
        "qualifiers": if qualifiers.is_empty() { Value::Null } else { Value::Object(qualifiers.into_iter().collect()) },
        "subpath": purl.subpath(),
    })
}

/// `None` when the case passes, otherwise a short reason.
fn run_case(case: &Value) -> Option<String> {
    let input = case["input"].as_str()?;
    let fails = case["expected_failure"].as_bool().unwrap_or(false);
    // A failure is a PURL that does not parse, or one its type does not allow.
    let parsed = Purl::parse(input)
        .map_err(|err| err.to_string())
        .and_then(|purl| {
            purl.check_type()
                .map(|()| purl)
                .map_err(|err| err.to_string())
        });
    match (case["test_type"].as_str()?, fails, &parsed) {
        (_, true, Err(_)) => None,
        (_, true, Ok(purl)) => Some(format!("accepted, canonical {}", purl.canonical())),
        (_, false, Err(err)) => Some(format!("rejected: {err}")),
        ("parse", false, Ok(purl)) => {
            let mut expected = case["expected_output"].clone();
            if expected["qualifiers"]
                .as_object()
                .is_some_and(|q| q.is_empty())
            {
                expected["qualifiers"] = Value::Null;
            }
            let canonical = Purl::parse(purl.canonical().as_str()).ok()?;
            let actual = components(&canonical);
            (actual != expected).then(|| format!("components {actual}"))
        }
        ("validate" | "roundtrip", false, Ok(purl)) => {
            let expected = opt(&case["expected_output"])?;
            (purl.canonical().as_str() != expected)
                .then(|| format!("canonical {}", purl.canonical()))
        }
        _ => None,
    }
}

/// Cases whose expected result needs a rule this module does not apply,
/// keyed by file, test type and input, with the reason. The type rules it
/// applies come from the data fields of `spec/purl/types/`; rules those
/// definitions give only in prose are not applied. Cases marked "upstream"
/// contradict another case or the type's definition.
const TYPE_SPECIFIC: &[(&str, &str, &str, &str)] = &[
    ("cpan-test.json", "parse", "pkg:cpan/LWP::UserAgent@6.7.6", "a rule the definition gives only in prose: a distribution name has no `::`"),
    ("cpan-test.json", "parse", "pkg:cpan/GDT/URI::PackageURL", "a rule the definition gives only in prose: a distribution name has no `::`"),
    ("cpan-test.json", "parse", "pkg:cpan/URI::PackageURL", "a rule the definition gives only in prose: a distribution name has no `::`"),
    ("gem-test.json", "parse", "pkg:gem/jruby-launcher@1.1.2?Platform=java", "upstream contradiction: the validate case for the same input expects the key lowercased"),
    ("git-test.json", "validate", "pkg:git/github/Package-url/purl-Spec@244fd47e07d1004f0aed9c", "upstream: the git definition marks namespace and name case-sensitive, but this case lowercases them"),
    ("git-test.json", "parse", "pkg:git/codeberg.org/forgejo/forgejo@a72d2c07cfca03b55371089de6aa230d8c951fa0", "a rule the definition gives only in prose: the namespace is the host and the name is the rest of the path"),
    ("git-test.json", "parse", "pkg:git/codeberg.org/forgejo/forgejo@a72d2c07cfca03b55371089de6aa230d8c951fa0#options/locale_readme.md", "a rule the definition gives only in prose: the namespace is the host and the name is the rest of the path"),
    ("mlflow-test.json", "validate", "pkg:mlflow/CreditFraud@3?repository_url=https://adb-5245952564735461.0.azuredatabricks.net/api/2.0/mlflow", "a rule the definition gives only in prose: the name is lowercased for Databricks servers only"),
    ("mlflow-test.json", "parse", "pkg:mlflow/CreditFraud@3?repository_url=https://adb-5245952564735461.0.azuredatabricks.net/api/2.0/mlflow", "a rule the definition gives only in prose: the name is lowercased for Databricks servers only"),
    ("swift-test.json", "parse", "pkg:swift/github.com/Alamofire/@5.4.3", "the generic rules drop the trailing `/`, as for every type, so the name is `Alamofire`; the suite expects no name"),
    ("pypi-test.json", "parse", "pkg:pypi/My.PyPI_.--._Package@1.11.1.dev1", "upstream: the expected subpath is a description sentence, not a subpath"),
    ("rpm-test.json", "parse", "pkg:Rpm/fedora/curl@7.50.3-1.fc25?Arch=i386&Distro=fedora-25", "upstream contradiction: the validate case for the same input expects the keys lowercased"),
];

#[test]
fn purl_spec_suite() {
    let mut failures = Vec::new();
    let mut passed = 0;
    let mut skipped = 0;
    for file in suite_files() {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let data: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        for case in data["tests"].as_array().unwrap() {
            if case["test_type"] == "build" || !case["input"].is_string() {
                skipped += 1;
                continue;
            }
            let description = case["description"].as_str().unwrap_or("");
            let input = case["input"].as_str().unwrap();
            let kind = case["test_type"].as_str().unwrap_or("");
            let listed = TYPE_SPECIFIC
                .iter()
                .any(|(f, k, i, _)| *f == name && *k == kind && *i == input);
            match (run_case(case), listed) {
                (None, false) => passed += 1,
                (Some(_), true) => skipped += 1,
                (None, true) => failures.push(format!(
                    "{name} [{kind}] {input} now passes; remove it from TYPE_SPECIFIC"
                )),
                (Some(reason), false) => failures.push(format!(
                    "{name} [{kind}] `{description}`: {input} -> {reason}"
                )),
            }
        }
    }
    eprintln!(
        "purl suite: {passed} passed, {skipped} skipped, {} failed",
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(passed > 100, "suite looks too small: {passed}");
}

#[test]
fn every_type_specific_entry_names_a_case_in_the_suite() {
    let mut cases = std::collections::BTreeSet::new();
    for file in suite_files() {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let data: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        for case in data["tests"].as_array().unwrap() {
            if let (Some(kind), Some(input)) = (case["test_type"].as_str(), case["input"].as_str())
            {
                cases.insert((name.clone(), kind.to_owned(), input.to_owned()));
            }
        }
    }
    for (file, kind, input, reason) in TYPE_SPECIFIC {
        assert!(!reason.is_empty());
        assert!(
            cases.contains(&((*file).to_owned(), (*kind).to_owned(), (*input).to_owned())),
            "{file} [{kind}] {input} is not in the suite"
        );
    }
}

#[test]
fn vendored_files_match_the_pinned_digests() {
    use sha2::{Digest, Sha256};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pinned = std::fs::read_to_string(root.join("spec/purl/PINNED.md")).unwrap();
    let mut checked = 0;
    for line in pinned
        .lines()
        .filter(|line| line.starts_with("| `spec/purl/"))
    {
        let cells: Vec<_> = line
            .split('|')
            .map(|cell| cell.trim().trim_matches('`'))
            .collect();
        let (path, digest) = (cells[1], cells[3]);
        let bytes = std::fs::read(root.join(path)).unwrap();
        let actual: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, digest, "{path}");
        checked += 1;
    }
    // Every vendored file is listed, so none can be added or edited unnoticed.
    let definitions = root.join("spec/purl/types");
    let on_disk = std::fs::read_dir(suite_dir().join("types"))
        .unwrap()
        .count()
        + std::fs::read_dir(definitions).unwrap().count()
        + 2;
    assert_eq!(
        checked, on_disk,
        "PINNED.md lists {checked} files, {on_disk} on disk"
    );
}

/// Every type definition against `Purl::canonical` and `Purl::check_type`:
/// the components marked `"case_sensitive": false` and only those are
/// lowercased, a required or prohibited namespace and required qualifiers are
/// enforced, and the `permitted_characters` patterns are the ones the module
/// implements.
#[test]
fn definitions_match_the_module() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/purl/types");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    assert_eq!(files.len(), 42);
    let mut patterns = Vec::new();
    for file in &files {
        let def: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        let ty = def["type"].as_str().unwrap();
        let insensitive = |key: &str| def[key]["case_sensitive"] == Value::Bool(false);
        for key in [
            "namespace_definition",
            "name_definition",
            "version_definition",
        ] {
            if let Some(pattern) = def[key]["permitted_characters"].as_str() {
                patterns.push(format!("{ty} {key} {pattern}"));
            }
        }
        // Names that pass the character rules of the two types that have them.
        let (name, version) = match ty {
            "chrome-extension" => ("Abcdefghijklmnopabcdefghijklmnop", "1.0"),
            _ => ("Name", "1.0RC"),
        };
        let purl = Purl::parse(&format!("pkg:{ty}/NameSpace/{name}@{version}#Sub/Path")).unwrap();
        let canonical = Purl::parse(purl.canonical().as_str()).unwrap();
        let fold = |text: &str, key: &str| {
            if insensitive(key) {
                text.to_ascii_lowercase()
            } else {
                text.to_owned()
            }
        };
        assert_eq!(
            canonical.namespace().unwrap(),
            fold("NameSpace", "namespace_definition"),
            "{ty}"
        );
        if ty != "pypi" {
            assert_eq!(canonical.name(), fold(name, "name_definition"), "{ty}");
        }
        assert_eq!(
            canonical.version().unwrap(),
            fold(version, "version_definition"),
            "{ty}"
        );
        assert_eq!(
            canonical.subpath().unwrap(),
            fold("Sub/Path", "subpath_definition"),
            "{ty}"
        );
        let any = [
            "namespace_definition",
            "name_definition",
            "version_definition",
            "subpath_definition",
        ]
        .iter()
        .any(|key| insensitive(key));
        let expected_rules = match (ty, any) {
            ("pypi", _) => PurlRules::Pypi,
            (_, true) => PurlRules::CaseInsensitive,
            (_, false) => PurlRules::Generic,
        };
        assert_eq!(purl.canonical().rules(), expected_rules, "{ty}");

        let required: Vec<String> = def["qualifiers_definition"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|q| q["requirement"] == "required")
            .map(|q| q["key"].as_str().unwrap().to_owned())
            .collect();
        let qualifiers: Vec<_> = required.iter().map(|key| format!("{key}=x")).collect();
        let query = if qualifiers.is_empty() {
            String::new()
        } else {
            format!("?{}", qualifiers.join("&"))
        };
        let check = |text: String| Purl::parse(&text).unwrap().check_type();
        let with_namespace = check(format!(
            "pkg:{ty}/ns/{name}@{version_ok}{query}",
            version_ok = "1.0"
        ));
        let without_namespace = check(format!("pkg:{ty}/{name}@1.0{query}"));
        match def["namespace_definition"]["requirement"].as_str() {
            Some("required") => {
                assert_eq!(with_namespace, Ok(()), "{ty}");
                assert_eq!(
                    without_namespace,
                    Err(PurlTypeError::NamespaceRequired),
                    "{ty}"
                );
            }
            Some("prohibited") => {
                assert_eq!(
                    with_namespace,
                    Err(PurlTypeError::NamespaceProhibited),
                    "{ty}"
                );
                assert_eq!(without_namespace, Ok(()), "{ty}");
            }
            _ => {
                assert_eq!(with_namespace, Ok(()), "{ty}");
                assert_eq!(without_namespace, Ok(()), "{ty}");
            }
        }
        let namespace = if def["namespace_definition"]["requirement"] == "prohibited" {
            ""
        } else {
            "ns/"
        };
        for key in &required {
            let others: Vec<_> = required
                .iter()
                .filter(|other| *other != key)
                .map(|other| format!("{other}=x"))
                .collect();
            let text = format!("pkg:{ty}/{namespace}{name}@1.0?{}", others.join("&"));
            let err = check(text).unwrap_err();
            assert_eq!(
                err.to_string(),
                format!("this Package URL type requires the qualifier `{key}`"),
                "{ty}"
            );
        }
    }
    assert_eq!(
        patterns,
        [
            "chrome-extension name_definition ^[a-p]{32}$",
            r"chrome-extension version_definition ^\d+(\.\d+){0,3}$",
            "pub name_definition ^[a-z0-9_]",
        ],
        "a definition's permitted_characters changed; update check_characters in src/purl.rs"
    );
}

#[test]
fn permitted_characters() {
    let check = |text: &str| Purl::parse(text).unwrap().check_type();
    let id = "dlpngalgnefjeiefhmpklpfiohadpglk";
    assert_eq!(check(&format!("pkg:chrome-extension/{id}@1.2.3.4")), Ok(()));
    assert_eq!(
        check(&format!("pkg:chrome-extension/{}", id.to_uppercase())),
        Ok(())
    );
    assert_eq!(
        check("pkg:chrome-extension/dogs"),
        Err(PurlTypeError::NameCharacters)
    );
    assert_eq!(
        check(&format!("pkg:chrome-extension/{id}@1.2.3.4.5")),
        Err(PurlTypeError::VersionCharacters)
    );
    assert_eq!(
        check(&format!("pkg:chrome-extension/{id}@1..2")),
        Err(PurlTypeError::VersionCharacters)
    );
    assert_eq!(check("pkg:pub/Flutter_Bloc@8.0.0"), Ok(()));
    assert_eq!(check("pkg:pub/-bloc"), Err(PurlTypeError::NameCharacters));
    // A type without a definition is not checked.
    assert_eq!(check("pkg:unknowntype/a/b/c"), Ok(()));
}
