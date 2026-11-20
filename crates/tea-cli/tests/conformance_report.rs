//! `libtea conformance` prints the same report as before the conformance code
//! moved into the CLI (`docs/cli-json.md`). The fixture was taken from the
//! 0.1.8 binary; a change to a case or to the shape must update it on purpose.

use std::path::Path;
use std::process::Command;

fn fixture() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/conformance-report.json"),
    )
    .unwrap()
}

#[test]
fn the_report_is_byte_identical_to_the_fixture_with_and_without_json() {
    for args in [&["conformance"][..], &["conformance", "--json"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_libtea"))
            .args(args)
            .env("RUST_LOG", "off")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            fixture(),
            "{args:?}"
        );
        assert!(output.stderr.is_empty(), "{args:?}");
    }
}
