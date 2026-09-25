use crate::{TsppFormatOptions, assert_format, assert_format_program, parse_first_expression};
use tspp_source::FileType;

/// Empty structs should collapse cleanly.
#[test]
fn test_format_struct_empty() {
    assert_format!(
        r#"struct Foo { }"#,
        r#"struct Foo {}"#,
        parse_first_expression,
        TsppFormatOptions::default()
    );
}

/// Malformed object properties should preserve their authored source.
#[test]
fn test_format_recovered_property() {
    assert_format!(
        "const value = { : 1, y: 2 }",
        "const value = { : 1, y: 2 }",
        parse_first_expression,
        TsppFormatOptions::default(),
    );
}

/// Struct fields should expand into one field per line.
#[test]
fn test_format_struct_with_fields() {
    assert_format!(
        r#"struct Foo { a: int32; b: boolean }"#,
        r#"struct Foo {
	a: int32;
	b: boolean;
}"#,
        parse_first_expression,
        TsppFormatOptions::default_tab()
    );
}

/// Field modifiers should stay attached to the field.
#[test]
fn test_format_class_with_abstract_override_field() {
    assert_format!(
        r#"class Foo { abstract override bar: int32 }"#,
        r#"class Foo {
	abstract override bar: int32;
}"#,
        parse_first_expression,
        TsppFormatOptions::default_tab()
    );
}

/// Struct member annotations should stay on their own line above the member.
#[test]
fn test_format_struct_with_decorated_field() {
    assert_format!(
        r#"struct User { @validate(minLength(1)) name: string }"#,
        r#"struct User {
	@validate(minLength(1))
	name: string;
}"#,
        parse_first_expression,
        TsppFormatOptions::default_tab()
    );
}

/// Quoted constructor names should normalize to constructor syntax.
#[test]
fn test_format_quoted_constructor_name() {
    assert_format_program!(
        r#"
[
  class {
    "constructor"(x: number, y: string) {}
  },
]
"#
        .trim_start(),
        r#"
[
  class {
    constructor(x: number, y: string) {}
  },
];
"#
        .trim_start(),
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// One single constructor parameter should stay inline.
#[test]
fn test_format_single_constructor_parameter_stays_inline() {
    assert_format_program!(
        r#"
class C {
  constructor(x: number) {}
}
"#
        .trim_start(),
        r#"
class C {
  constructor(x: number) {}
}
"#
        .trim_start(),
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Static `new` methods should keep generic parameters attached to the key.
#[test]
fn test_format_static_new_method_generic_parameters() {
    assert_format_program!(
        r#"
extension<T> of Set<T> {
  static new<T>(): Set<T> { undefined! }
}
"#
        .trim_start(),
        r#"
extension<T> of Set<T> {
  static new<T>(): Set<T> {
    undefined!
  }
}
"#
        .trim_start(),
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// String-named class fields should preserve their quoted spelling.
#[test]
fn test_format_class_string_keys_preserve_quotes() {
    assert_format_program!(
        r#"class Example { "a" = 1; "needs-quotes" = 2; }
"#,
        r#"class Example {
  "a" = 1;
  "needs-quotes" = 2;
}
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}
