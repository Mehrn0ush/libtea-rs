//! The Package URL test suite (`spec/purl/`, purl-spec v1.1.1) against
//! `libtea_core::purl`.
//!
//! `parse` cases compare components, `validate` cases compare the canonical
//! string, and `expected_failure` parse cases must be rejected. `build` cases
//! construct a PURL from components, which this module does not offer, so they
//! are skipped.
//!
//! The parser applies only the rules ECMA-427 gives for every type. Cases that
//! need a type-specific rule (case folding, character rules, required or
//! prohibited components) are listed in `TYPE_SPECIFIC` with that reason; the
//! test fails if one of them starts passing, so the list stays honest.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use libtea_core::purl::Purl;
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
    let parsed = Purl::parse(input);
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
            let actual = components(purl);
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

/// Cases whose expected result needs a rule this generic parser does not
/// apply, keyed by file, test type and input, with the rule. Two are not
/// type rules but cases the suite contradicts elsewhere (marked "upstream").
const TYPE_SPECIFIC: &[(&str, &str, &str, &str)] = &[
    ("bitbucket-test.json", "validate", "pkg:bitbucket/birKenfeld/pyGments-main@244fd47e07d1014f0aed9c", "namespace and name are case-insensitive (lowercased)"),
    ("bitbucket-test.json", "parse", "pkg:bitbucket/birKenfeld/pyGments-main@244fd47e07d1014f0aed9c", "namespace and name are case-insensitive (lowercased)"),
    ("brew-test.json", "parse", "pkg:brew/SQLite@3.43.2", "namespace and name are case-insensitive (lowercased)"),
    ("brew-test.json", "validate", "pkg:brew/SQLite@3.43.2", "namespace and name are case-insensitive (lowercased)"),
    ("brew-test.json", "parse", "pkg:brew/Homebrew/Core/sqlite@3.43.2", "namespace and name are case-insensitive (lowercased)"),
    ("brew-test.json", "validate", "pkg:brew/Homebrew/Core/sqlite@3.43.2", "namespace and name are case-insensitive (lowercased)"),
    ("chrome-extension-test.json", "parse", "pkg:chrome-extension/44444algnefjeiefhmpklpfiohadpglk", "type rules for name and version characters"),
    ("chrome-extension-test.json", "parse", "pkg:chrome-extension/dogs", "type rules for name and version characters"),
    ("chrome-extension-test.json", "parse", "pkg:chrome-extension/dlpngalgnefjeiefhmpklpfiohadpglk@1.2.3.4.5", "type rules for name and version characters"),
    ("chrome-extension-test.json", "parse", "pkg:chrome-extension/dlpngalgnefjeiefhmpklpfiohadpglk@1.2.3-beta", "type rules for name and version characters"),
    ("composer-test.json", "validate", "pkg:composer/Laravel/Laravel@5.5.0", "namespace and name are case-insensitive (lowercased)"),
    ("composer-test.json", "parse", "pkg:composer/Laravel/Laravel@5.5.0", "namespace and name are case-insensitive (lowercased)"),
    ("cpan-test.json", "parse", "pkg:cpan/LWP::UserAgent@6.7.6", "type rules for module and distribution names"),
    ("cpan-test.json", "parse", "pkg:cpan/GDT/URI::PackageURL", "type rules for module and distribution names"),
    ("cpan-test.json", "parse", "pkg:cpan/URI::PackageURL", "type rules for module and distribution names"),
    ("gem-test.json", "parse", "pkg:gem/jruby-launcher@1.1.2?Platform=java", "upstream contradiction: the validate case for the same input expects the key lowercased"),
    ("git-test.json", "validate", "pkg:git/github/Package-url/purl-Spec@244fd47e07d1004f0aed9c", "type rules for namespace, name and case"),
    ("git-test.json", "parse", "pkg:git/codeberg.org/forgejo/forgejo@a72d2c07cfca03b55371089de6aa230d8c951fa0", "type rules for namespace, name and case"),
    ("git-test.json", "parse", "pkg:git/codeberg.org/forgejo/forgejo@a72d2c07cfca03b55371089de6aa230d8c951fa0#options/locale_readme.md", "type rules for namespace, name and case"),
    ("github-test.json", "validate", "pkg:github/Package-url/purl-Spec@244fd47e07d1004f0aed9c", "namespace and name are case-insensitive (lowercased)"),
    ("github-test.json", "parse", "pkg:github/Package-url/purl-Spec@244fd47e07d1004f0aed9c", "namespace and name are case-insensitive (lowercased)"),
    ("huggingface-test.json", "validate", "pkg:huggingface/EleutherAI/gpt-neo-1.3B@797174552AE47F449AB70B684CABCB6603E5E85E", "namespace is case-insensitive; version is lowercased"),
    ("huggingface-test.json", "parse", "pkg:huggingface/EleutherAI/gpt-neo-1.3B@797174552AE47F449AB70B684CABCB6603E5E85E", "namespace is case-insensitive; version is lowercased"),
    ("julia-test.json", "parse", "pkg:julia/Dates", "the uuid qualifier is required"),
    ("mlflow-test.json", "validate", "pkg:mlflow/CreditFraud@3?repository_url=https://adb-5245952564735461.0.azuredatabricks.net/api/2.0/mlflow", "name is case-insensitive (lowercased)"),
    ("mlflow-test.json", "parse", "pkg:mlflow/CreditFraud@3?repository_url=https://adb-5245952564735461.0.azuredatabricks.net/api/2.0/mlflow", "name is case-insensitive (lowercased)"),
    ("otp-test.json", "parse", "pkg:otp/namespace/hex@2.1.1", "type rules for the namespace"),
    ("pypi-test.json", "validate", "pkg:PYPI/Django_package@1.11.1.dev1", "name is lowercased and runs of `-`, `_`, `.` become `-`"),
    ("pypi-test.json", "parse", "pkg:PYPI/Django_package@1.11.1.dev1", "name is lowercased and runs of `-`, `_`, `.` become `-`"),
    ("pypi-test.json", "parse", "pkg:pypi/friendly.bard@1.0", "name is lowercased and runs of `-`, `_`, `.` become `-`"),
    ("pypi-test.json", "parse", "pkg:pypi/friendly--bard@1.0", "name is lowercased and runs of `-`, `_`, `.` become `-`"),
    ("pypi-test.json", "parse", "pkg:pypi/friendly__bard@1.0", "name is lowercased and runs of `-`, `_`, `.` become `-`"),
    ("pypi-test.json", "parse", "pkg:pypi/My.PyPI_.--._Package@1.11.1.dev1", "name is lowercased and runs of `-`, `_`, `.` become `-`"),
    ("pypi-test.json", "validate", "pkg:pypi/My.PyPI_.--._Package@1.11.1.dev1", "name is lowercased and runs of `-`, `_`, `.` become `-`"),
    ("rpm-test.json", "parse", "pkg:Rpm/fedora/curl@7.50.3-1.fc25?Arch=i386&Distro=fedora-25", "upstream contradiction: the validate case for the same input expects the keys lowercased"),
    ("swift-test.json", "parse", "pkg:swift/Alamofire@5.4.3", "namespace is required"),
    ("swift-test.json", "parse", "pkg:swift/github.com/Alamofire/@5.4.3", "namespace is required"),
    ("vcpkg-test.json", "parse", "pkg:vcpkg/boost/asio@1.84.0", "namespace is prohibited"),
    ("vscode-extension-test.json", "parse", "pkg:vscode-extension/java@1.46.2025091308", "namespace is required"),
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
    let on_disk = std::fs::read_dir(suite_dir().join("types"))
        .unwrap()
        .count()
        + 2;
    assert_eq!(
        checked, on_disk,
        "PINNED.md lists {checked} files, {on_disk} on disk"
    );
}
