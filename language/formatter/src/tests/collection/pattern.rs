use crate::{DestackFormatOptions, assert_format};

#[test]
fn test_format_pattern_wildcard() {
    assert_format!("_", "_", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_reference_wildcard() {
    assert_format!("&_", "&_", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_reference_literal() {
    assert_format!("&1", "&1", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_dereference_binding() {
    assert_format!("*value", "*value", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_dereference_range() {
    assert_format!("*0..10", "*0..10", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_dereference_tuple() {
    assert_format!("*(x, y)", "*(x, y)", |p| p.eat_pattern());
}

#[test]
fn test_format_pattern_dereference_sequence() {
    assert_format!("*[head, ...tail]", "*[head, ...tail]", |p| {
        p.eat_pattern()
    });
}

#[test]
fn test_format_pattern_dereference_tagged_object() {
    assert_format!("*Point { x, y }", "*Point { x, y }", |p| {
        p.eat_pattern()
    });
}

#[test]
fn test_format_pattern_dereference_before_borrow() {
    assert_format!("*&readonly inner", "*&readonly inner", |p| {
        p.eat_pattern()
    });
}

#[test]
fn test_format_pattern_borrow_before_dereference() {
    assert_format!("&*inner", "&*inner", |p| p.eat_pattern());
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
fn test_format_newtype_object_pattern() {
    assert_format!(
        "Shape.Line({ start: Point { x, y }, end })",
        r#"Shape.Line({
    start: Point { x, y },
    end,
})"#,
        |p| p.eat_pattern(),
        DestackFormatOptions::default_with_line_width(30)
    );
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
