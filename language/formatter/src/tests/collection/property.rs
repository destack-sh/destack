use crate::{DestackFormatOptions, assert_format, assert_format_program};
use destack_source::{FileType, LanguageType};

/// Empty structs should collapse cleanly.
#[test]
fn test_format_struct_empty() {
    assert_format!(
        r#"struct Foo { }"#,
        r#"struct Foo {}"#,
        crate::parse_first_expression,
        DestackFormatOptions::default()
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
        crate::parse_first_expression,
        DestackFormatOptions::default_tab()
    );
}

/// TypeScript field modifiers should stay attached to the field.
#[test]
fn test_format_class_with_abstract_override_field() {
    assert_format!(
        r#"class Foo { abstract override bar: int32 }"#,
        r#"class Foo {
	abstract override bar: int32;
}"#,
        crate::parse_first_expression,
        DestackFormatOptions {
            language_type: LanguageType::TypeScript,
            ..DestackFormatOptions::default_tab()
        }
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
        crate::parse_first_expression,
        DestackFormatOptions::default_tab()
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
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
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
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
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
        FileType::Destack,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// JavaScript class fields should drop unnecessary key quotes.
#[test]
fn test_format_javascript_class_quote_props_as_needed() {
    assert_format_program!(
        r#"class Example { "a" = 1; "needs-quotes" = 2; }
"#,
        r#"class Example {
  a = 1;
  "needs-quotes" = 2;
}
"#,
        FileType::JavaScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}
