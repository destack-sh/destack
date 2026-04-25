use crate::{DestackFormatOptions, assert_format};

#[test]
fn test_format_pattern_wildcard() {
    assert_format!("_", "_", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_reference() {
    assert_format!("&_", "&_", |p| p.eat_pattern());

    assert_format!("&1", "&1", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_must() {
    assert_format!("T!", "T!", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_identifier() {
    assert_format!("x", "x", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_path() {
    assert_format!("MyEnum.A", "MyEnum.A", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_tuple() {
    assert_format!("(x: 1, 2, ...)", "(x: 1, 2, ...)", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_tuple_with_path() {
    assert_format!("Result.Success(_, ...)", "Result.Success(_, ...)", |p| p
        .eat_pattern());
}

#[test]
fn test_format_pattern_slice() {
    assert_format!("[1, 2, ...]", "[1, 2, ...]", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_union() {
    assert_format!("1 | 2 | 3 | 4 | 5", "1 | 2 | 3 | 4 | 5", |p| p
        .eat_pattern());
}

#[test]
fn test_format_pattern_array_rest_disallows_trailing_comma() {
    assert_format!(
        "[a, ...rest]",
        r#"[
    a,
    ...rest
]"#,
        |p| p.eat_pattern(),
        DestackFormatOptions::default_with_line_width(1)
    );
}

#[test]
fn test_format_pattern_tuple_rest_disallows_trailing_comma() {
    assert_format!(
        "(a, ...rest)",
        r#"(
    a,
    ...rest
)"#,
        |p| p.eat_pattern(),
        DestackFormatOptions::default_with_line_width(1)
    );
}
