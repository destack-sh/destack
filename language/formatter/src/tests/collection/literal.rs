use crate::{
    TsppFormatOptions, assert_format, assert_format_program, assert_format_roundtrip,
    assert_format_roundtrip_with_file_type, parse_first_expression,
};
use tspp_repository::TrailingComma;
use tspp_source::FileType;

/// Character literals should keep their single-quoted spelling.
#[test]
fn test_format_character_literal_keeps_single_quotes() {
    assert_format_roundtrip_with_file_type(
        r#"'a'"#,
        r#"'a'"#,
        FileType::Tspp,
        parse_first_expression,
        TsppFormatOptions::default(),
    );
}

/// Embedded double quotes should stay escaped once.
#[test]
fn test_format_string_literal_escapes_embedded_target_quote() {
    assert_format_roundtrip_with_file_type(
        r#""\"1\"""#,
        r#""\"1\"""#,
        FileType::Tspp,
        parse_first_expression,
        TsppFormatOptions::default(),
    );
}

/// TS++ strings should not use character literal quotes.
#[test]
fn test_format_tspp_string_literal_keeps_double_quotes() {
    assert_format_roundtrip_with_file_type(
        r#""say \"hello\"""#,
        r#""say \"hello\"""#,
        FileType::Tspp,
        parse_first_expression,
        TsppFormatOptions::default(),
    );
}

/// Template literals with interpolation should stay stable.
#[test]
fn test_format_template_literal_one_interpolation() {
    assert_format!(
        r#"tagged`hello ${name}`"#,
        r#"tagged`hello ${name}`"#,
        parse_first_expression
    );
}

/// Ternary interpolations in one-line templates should remain inline and idempotent.
#[test]
fn test_format_template_literal_ternary_interpolation_stays_inline_roundtrip() {
    assert_format_roundtrip!(
        r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
        r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
        FileType::Tspp,
        parse_first_expression,
    );
}

/// Long strings should not be forcibly broken.
#[test]
fn test_format_long_string_not_broken() {
    assert_format!(
        r#""This is a very long string that exceeds the line width but should not be broken""#,
        r#""This is a very long string that exceeds the line width but should not be broken""#,
        parse_first_expression,
        TsppFormatOptions::default_with_line_width(40)
    );
}

/// Paths with multiple segments should stay stable.
#[test]
fn test_format_path_multiple_segments() {
    assert_format!(
        r#"tspp.geometry.math"#,
        r#"tspp.geometry.math"#,
        |p| p.parse_path(),
        TsppFormatOptions::default()
    );
}

/// Singleton tuple literals should keep their required trailing comma.
#[test]
fn test_format_singleton_tuple_literal() {
    assert_format_program!(
        r#"const value = (1,)
"#,
        r#"const value = (1,);
"#,
        FileType::Tspp,
    );
}

/// Sparse arrays should preserve every elided element.
#[test]
fn test_format_sparse_array_elisions() {
    assert_format_program!(
        r#"const leading=[,value]
const repeated=[,,value]
const middle=[first,,third]
const trailing=[first,,]
"#,
        r#"const leading = [, value];
const repeated = [, , value];
const middle = [first, , third];
const trailing = [first, ,];
"#,
        FileType::Tspp,
    );
}

/// Closing elisions should retain their comma when optional trailing commas are disabled.
#[test]
fn test_format_sparse_array_requires_closing_elision_comma() {
    assert_format_program!(
        "const values=[first,,]\n",
        "const values = [first, ,];\n",
        FileType::Tspp,
        TsppFormatOptions {
            trailing_comma: TrailingComma::None,
            ..TsppFormatOptions::default()
        },
    );
}

/// Multiline sparse arrays should keep elisions as empty entries.
#[test]
fn test_format_multiline_sparse_array() {
    assert_format_program!(
        r#"const values = [
  first,
  ,
  third
]
"#,
        r#"const values = [
    first,
    ,
    third,
];
"#,
        FileType::Tspp,
    );
}

/// Sparse array comments should remain stable around elision separators.
#[test]
fn test_format_sparse_array_comment_separator() {
    assert_format_program!(
        r#"const inline=[first,/* unavailable */,third]
const multiline=[first,
// unavailable
,third]
"#,
        r#"const inline = [first, , /* unavailable */ third];
const multiline = [
    first,
    ,
    // unavailable
    third,
];
"#,
        FileType::Tspp,
    );
}
