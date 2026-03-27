use crate::{
    DestackFormatOptions, TestFormatter, assert_format, assert_format_roundtrip_with_file_type,
};
use destack_source::FileType;

/// Multi-char strings use double quotes in semantic mode.
#[test]
fn test_format_string_literal_multi_char() {
    assert_format!("'hello'", "\"hello\"", |p| p
        .eat_expression(Default::default()));
}

/// Single-char strings use single quotes in semantic mode.
#[test]
fn test_format_string_literal_single_char() {
    assert_format!("\"a\"", "'a'", |p| p.eat_expression(Default::default()));
}

/// Empty strings use double quotes in semantic mode.
#[test]
fn test_format_string_literal_empty() {
    assert_format!("''", "\"\"", |p| p.eat_expression(Default::default()));
}

#[test]
fn test_format_string_literal_escapes_embedded_target_quote() {
    assert_format!("'\"1\"'", r#""\"1\"""#, |p| p
        .eat_expression(Default::default()));
}

#[test]
fn test_format_string_literal_does_not_double_escape_target_quote() {
    assert_format!(r#""\"1\"""#, r#""\"1\"""#, |p| p
        .eat_expression(Default::default()));
}

/// Formats a template literal string with no interpolation.
#[test]
fn test_format_template_literal_plain() {
    let source = "`hello`";
    assert_format!(source, source, |p| p.eat_expression(Default::default()));
}

/// Formats a template literal string with one interpolation.
#[test]
fn test_format_template_literal_one_interpolation() {
    let source = "tagged`hello ${name}`";
    assert_format!(source, source, |p| p.eat_expression(Default::default()));
}

/// Formats a template literal string where the entire content is interpolation.
#[test]
fn test_format_template_literal_all_interpolation() {
    let source = "sql`${stmt}`";
    assert_format!(source, source, |p| p.eat_expression(Default::default()));
}

/// Formats a template literal string with multiple adjacent interpolations.
#[test]
fn test_format_template_literal_adjacent_interpolations() {
    let source = "`${start}${middle}${end}`";
    assert_format!(source, source, |p| p.eat_expression(Default::default()));
}

/// Formats a template literal with a complex SQL query and interpolation.
#[test]
fn test_format_template_literal_complex_sql() {
    let source =
        r#"sql.stmt`SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`"#;
    assert_format!(source, source, |p| p.eat_expression(Default::default()));
}

/// Ternary interpolations in one-line templates should remain inline and idempotent.
#[test]
fn test_format_template_literal_ternary_interpolation_stays_inline_roundtrip() {
    assert_format_roundtrip_with_file_type(
        r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
        r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Long strings are NOT broken even when they exceed line width (like Prettier).
#[test]
fn test_format_long_string_not_broken() {
    let source =
        r#""This is a very long string that exceeds the line width but should not be broken""#;
    assert_format!(
        source,
        source,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(40)
    );
}

/// Long template literals are NOT broken even when they exceed line width.
#[test]
fn test_format_long_template_literal_not_broken() {
    let source = r#"`This is a very long template literal that exceeds the line width but should not be broken`"#;
    assert_format!(
        source,
        source,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(40)
    );
}

#[test]
fn test_format_path_short() {
    assert_format!(
        "destack",
        "destack",
        |p| p.eat_path(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_path_multiple_segments() {
    assert_format!(
        "destack.geometry.math",
        "destack.geometry.math",
        |p| p.eat_path(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_path_with_overlong_line() {
    assert_format!(
        "destack.geometry.math.vector.point",
        "destack.geometry.math.vector.point",
        |p| p.eat_path(),
        DestackFormatOptions::default().with_line_width(20)
    );
}
