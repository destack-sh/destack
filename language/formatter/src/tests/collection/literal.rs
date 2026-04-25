use crate::{DestackFormatOptions, assert_format, assert_format_roundtrip};
use destack_source::FileType;

/// Multi-char strings should normalize to double quotes in semantic mode.
#[test]
fn test_format_string_literal_multi_char() {
    assert_format!(r#"'hello'"#, r#""hello""#, |p| p
        .eat_expression(Default::default()));
}

/// Embedded target quotes should stay escaped once.
#[test]
fn test_format_string_literal_escapes_embedded_target_quote() {
    assert_format!(r#"'\"1\"'"#, r#"'"1"'"#, |p| p
        .eat_expression(Default::default()));
}

/// Template literals with interpolation should stay stable.
#[test]
fn test_format_template_literal_one_interpolation() {
    assert_format!(
        r#"tagged`hello ${name}`"#,
        r#"tagged`hello ${name}`"#,
        |p| p.eat_expression(Default::default())
    );
}

/// Ternary interpolations in one-line templates should remain inline and idempotent.
#[test]
fn test_format_template_literal_ternary_interpolation_stays_inline_roundtrip() {
    assert_format_roundtrip!(
        r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
        r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
    );
}

/// Long strings should not be forcibly broken.
#[test]
fn test_format_long_string_not_broken() {
    assert_format!(
        r#""This is a very long string that exceeds the line width but should not be broken""#,
        r#""This is a very long string that exceeds the line width but should not be broken""#,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(40)
    );
}

/// Paths with multiple segments should stay stable.
#[test]
fn test_format_path_multiple_segments() {
    assert_format!(
        r#"destack.geometry.math"#,
        r#"destack.geometry.math"#,
        |p| p.eat_path(),
        DestackFormatOptions::default()
    );
}
