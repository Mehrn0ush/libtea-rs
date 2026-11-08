//! `libtea_core::escape_control_chars`.

use libtea_core::escape_control_chars as escape;

#[test]
fn controls_become_unicode_escapes_and_other_text_is_kept() {
    assert_eq!(
        escape("plain\ttab\nnewline é 日本 \u{1f600}"),
        "plain\ttab\nnewline é 日本 \u{1f600}"
    );
    assert_eq!(escape("\u{1b}[31mred"), "\\u001b[31mred");
    assert_eq!(escape("\u{1b}]0;t\u{7}"), "\\u001b]0;t\\u0007");
    assert_eq!(escape("a\rb\u{7f}"), "a\\u000db\\u007f");
    assert_eq!(
        escape("\u{9b}2J\u{9d}0;t\u{85}"),
        "\\u009b2J\\u009d0;t\\u0085"
    );
    assert_eq!(
        escape("\u{202a}\u{202e}\u{2066}\u{2069}"),
        "\\u202a\\u202e\\u2066\\u2069"
    );
    assert_eq!(escape(""), "");
}

#[test]
fn no_scalar_value_survives_as_a_control() {
    let all: String = (0u32..=0x10FFFF).filter_map(char::from_u32).collect();
    let out = escape(&all);
    assert!(out.chars().all(|c| !(c.is_control()
        || ('\u{202a}'..='\u{202e}').contains(&c)
        || ('\u{2066}'..='\u{2069}').contains(&c))
        || c == '\t'
        || c == '\n'));
}

#[test]
fn escaping_twice_changes_nothing() {
    let once = escape("x\u{1b}[0m\u{9b}y");
    assert_eq!(escape(&once), once);
}
