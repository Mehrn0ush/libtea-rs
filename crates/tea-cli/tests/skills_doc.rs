//! `SKILLS.md`, `README.md` and the crate READMEs must agree with the CLI:
//! the exit-status table with `docs/cli-exit-codes.md` and the constants in
//! `src/main.rs`, and every `libtea` subcommand and flag they show must exist.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// `(code, name)` rows of a markdown table whose first cell is an integer.
fn table_rows(text: &str) -> BTreeMap<u8, String> {
    let mut rows = BTreeMap::new();
    for line in text.lines() {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() < 2 {
            continue;
        }
        if let Ok(code) = cells[0].parse::<u8>() {
            rows.insert(code, cells[1].to_owned());
        }
    }
    rows
}

/// `pub const NAME: u8 = N;` inside `mod exit` of the CLI source.
fn exit_constants() -> BTreeMap<u8, String> {
    let source = read("crates/tea-cli/src/main.rs");
    let start = source.find("mod exit {").expect("mod exit");
    let body = &source[start..];
    let body = &body[..body.find("\n}\n").expect("end of mod exit")];
    let mut found = BTreeMap::new();
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("pub const ") {
            let (name, value) = rest.split_once(": u8 = ").expect("const shape");
            let value: u8 = value.trim_end_matches(';').parse().expect("number");
            found.insert(value, name.to_owned());
        }
    }
    found
}

fn doc_name(constant: &str) -> &'static str {
    match constant {
        "OK" => "success",
        "FAILURE" => "failure",
        "USAGE" => "usage",
        "AUTH" => "auth",
        "WEAK_CHECKSUMS" => "weak-only",
        "NOT_FOUND" => "not found",
        "INTEGRITY" => "integrity",
        "CHANGES_DETECTED" => "changes detected",
        other => panic!("exit constant `{other}` has no documented name; document it"),
    }
}

#[test]
fn exit_status_tables_match_the_code_constants() {
    let from_code: BTreeMap<u8, String> = exit_constants()
        .into_iter()
        .map(|(code, name)| (code, doc_name(&name).to_owned()))
        .collect();
    assert_eq!(from_code.len(), 8, "{from_code:?}");
    for file in ["docs/cli-exit-codes.md", "SKILLS.md"] {
        let rows = table_rows(&read(file));
        assert_eq!(rows, from_code, "{file} disagrees with the exit constants");
    }
}

#[test]
fn skills_guide_stays_short_and_keeps_the_security_defaults() {
    let text = read("SKILLS.md");
    let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let flat = squash(&text);
    assert!(
        text.lines().count() < 150,
        "SKILLS.md must stay under 150 lines"
    );
    for needed in [
        "Do not add `--allow-origin` for a",
        "Do not add `--allow-weak-checksums`",
        "never print, log or",
        "not authenticity",
        "does not verify signatures",
    ] {
        assert!(flat.contains(&squash(needed)), "SKILLS.md lacks `{needed}`");
    }
    // Nothing may recommend a weaker setting. A bullet or paragraph that names
    // one of these flags has to be a prohibition.
    let mut blocks: Vec<String> = Vec::new();
    for line in text.lines() {
        match blocks.last_mut() {
            Some(last) if !line.starts_with("- ") && !line.trim().is_empty() => {
                last.push(' ');
                last.push_str(line.trim());
            }
            _ => blocks.push(line.trim().to_owned()),
        }
    }
    for block in blocks {
        let block = block.to_ascii_lowercase();
        if block.contains("--allow-origin")
            || block.contains("--allow-weak-checksums")
            || block.contains("--allow-discovered-credentials")
        {
            assert!(
                block.contains("do not") || block.contains("never") || block.contains("only for"),
                "could read as advice to weaken a default: {block}"
            );
        }
    }
}

fn help(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(args)
        .arg("--help")
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Lines inside ```sh fences.
fn shell_lines(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            inside = !inside && line.trim_start().starts_with("```sh");
            continue;
        }
        if inside {
            out.push(line.to_owned());
        }
    }
    out
}

#[test]
fn documented_libtea_commands_and_flags_exist() {
    const SUBCOMMANDS: [&str; 10] = [
        "spec",
        "parse-tei",
        "conformance",
        "discover",
        "product-release",
        "collections",
        "download",
        "sync",
        "diff",
        "cache",
    ];
    let mut helps: BTreeMap<&str, String> = BTreeMap::new();
    for sub in SUBCOMMANDS {
        helps.insert(sub, help(&[sub]));
    }
    helps.insert("cache stat", help(&["cache", "stat"]));
    helps.insert("cache clear", help(&["cache", "clear"]));
    let all_help: String = helps.values().cloned().collect::<Vec<_>>().join("\n");
    for file in [
        "README.md",
        "SKILLS.md",
        "crates/tea-core/README.md",
        "crates/tea-client/README.md",
    ] {
        let text = read(file);
        let mut lines = shell_lines(&text);
        // Continuation lines belong to the command that started them.
        let mut joined: Vec<String> = Vec::new();
        for line in lines.drain(..) {
            match joined.last_mut() {
                Some(last) if last.ends_with('\\') => {
                    last.pop();
                    last.push(' ');
                    last.push_str(line.trim());
                }
                _ => joined.push(line.trim().to_owned()),
            }
        }
        for command in joined {
            let words: Vec<&str> = command.split_whitespace().collect();
            let Some(at) = words.iter().position(|w| *w == "libtea") else {
                continue;
            };
            let after = &words[at + 1..];
            let Some(sub) = after.iter().find(|w| !w.starts_with('-')) else {
                continue;
            };
            assert!(
                SUBCOMMANDS.contains(sub),
                "{file}: unknown libtea subcommand `{sub}` in `{command}`"
            );
            let help = if *sub == "cache" {
                all_help.clone()
            } else {
                helps[sub].clone()
            };
            for word in after.iter().filter(|w| w.starts_with("--")) {
                let flag = word.split('=').next().unwrap();
                assert!(
                    help.contains(flag) || flag == "--help",
                    "{file}: `{flag}` is not a flag of `libtea {sub}` in `{command}`"
                );
            }
        }
    }
}
