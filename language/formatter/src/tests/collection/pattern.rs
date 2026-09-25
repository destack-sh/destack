use crate::{TsppFormatOptions, assert_format, assert_format_roundtrip};
use tspp_source::FileType;

#[test]
fn test_format_pattern_wildcard() {
    assert_format!("_", "_", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_reference_wildcard() {
    assert_format!("&_", "&_", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_reference_literal() {
    assert_format!("&1", "&1", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_dereference_binding() {
    assert_format!("*value", "*value", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_dereference_range() {
    assert_format!("*0..10", "*0..10", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_dereference_tuple() {
    assert_format!("*(x, y)", "*(x, y)", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_dereference_sequence() {
    assert_format!("*[head, ...tail]", "*[head, ...tail]", |p| {
        p.parse_pattern_fragment()
    });
}

#[test]
fn test_format_pattern_dereference_tagged_object() {
    assert_format!("*Point { x, y }", "*Point { x, y }", |p| {
        p.parse_pattern_fragment()
    });
}

#[test]
fn test_format_pattern_dereference_before_borrow() {
    assert_format!("*&readonly inner", "*&readonly inner", |p| {
        p.parse_pattern_fragment()
    });
}

#[test]
fn test_format_pattern_borrow_before_dereference() {
    assert_format!("&*inner", "&*inner", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_borrow_chain_compact() {
    assert_format_roundtrip!("& &item", "&&item", FileType::Tspp, |p| p
        .parse_pattern_fragment());
    assert_format_roundtrip!("&&item", "&&item", FileType::Tspp, |p| p
        .parse_pattern_fragment());
}

#[test]
fn test_format_pattern_move_chain_compact() {
    assert_format_roundtrip!("^ ^item", "^^item", FileType::Tspp, |p| p
        .parse_pattern_fragment());
}

#[test]
fn test_format_pattern_mixed_borrow_move_chain_compact() {
    assert_format_roundtrip!("& ^item", "&^item", FileType::Tspp, |p| p
        .parse_pattern_fragment());
}

#[test]
fn test_format_pattern_mixed_move_borrow_chain_compact() {
    assert_format_roundtrip!("^ &item", "^&item", FileType::Tspp, |p| p
        .parse_pattern_fragment());
}

#[test]
fn test_format_pattern_prefix_comments() {
    assert_format_roundtrip!(
        "& /* borrowed */ item",
        "& /* borrowed */ item",
        FileType::Tspp,
        |p| p.parse_pattern_fragment()
    );
    assert_format_roundtrip!(
        "* &readonly /* read */ item",
        "*&readonly /* read */ item",
        FileType::Tspp,
        |p| p.parse_pattern_fragment()
    );
}

#[test]
fn test_format_pattern_must() {
    assert_format!("T!", "T!", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_identifier() {
    assert_format!("x", "x", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_path() {
    assert_format!("MyEnum.A", "MyEnum.A", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_tuple() {
    assert_format!("(x: 1, 2, ...)", "(x: 1, 2, ...)", |p| p
        .parse_pattern_fragment());
}

#[test]
fn test_format_pattern_singleton_tuple_requires_comma() {
    assert_format!("(x)", "(x,)", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_tuple_with_path() {
    assert_format!("Result.Success(_, ...)", "Result.Success(_, ...)", |p| p
        .parse_pattern_fragment());
}

#[test]
fn test_format_pattern_nominal_singleton_tuple_omits_comma() {
    assert_format!("Result.Success(x)", "Result.Success(x)", |p| p
        .parse_pattern_fragment());
}

#[test]
fn test_format_newtype_object_pattern() {
    assert_format!(
        "Shape.Line({ start: Point { x, y }, end })",
        r#"Shape.Line({
    start: Point { x, y },
    end,
})"#,
        |p| p.parse_pattern_fragment(),
        TsppFormatOptions::default_with_line_width(30)
    );
}

#[test]
fn test_format_pattern_slice() {
    assert_format!("[1, 2, ...]", "[1, 2, ...]", |p| p.parse_pattern_fragment());
}

#[test]
fn test_format_pattern_union() {
    assert_format!("1 | 2 | 3 | 4 | 5", "1 | 2 | 3 | 4 | 5", |p| p
        .parse_pattern_fragment());
}

#[test]
fn test_format_pattern_array_rest_disallows_trailing_comma() {
    assert_format!(
        "[a, ...rest]",
        r#"[
    a,
    ...rest
]"#,
        |p| p.parse_pattern_fragment(),
        TsppFormatOptions::default_with_line_width(1)
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
        |p| p.parse_pattern_fragment(),
        TsppFormatOptions::default_with_line_width(1)
    );
}
