use crate::{DestackFormatOptions, assert_format, assert_format_program};
use destack_source::{FileType, LanguageType};

/// Empty structs should collapse cleanly.
#[test]
fn test_format_struct_empty() {
    assert_format!(
        r#"struct Foo { }"#,
        r#"struct Foo {}"#,
        |p| p.eat_expression(Default::default()),
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
        |p| p.eat_expression(Default::default()),
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
        |p| p.eat_expression(Default::default()),
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
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab()
    );
}

/// Quoted constructor parameter properties should expand into one parameter per line.
#[test]
fn test_format_constructor_parameter_properties_expand_for_quoted_constructor_name() {
    assert_format_program!(
        r#"
[
  class {
    "constructor"(protected x: number, private y: string) {}
  },
]
"#
        .trim_start(),
        r#"
[
  class {
    constructor(
      protected x: number,
      private y: string,
    ) {}
  },
];
"#
        .trim_start(),
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// One single constructor parameter property should stay inline.
#[test]
fn test_format_single_constructor_parameter_property_stays_inline() {
    assert_format_program!(
        r#"
class C {
  constructor(private x: number) {}
}
"#
        .trim_start(),
        r#"
class C {
  constructor(private x: number) {}
}
"#
        .trim_start(),
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// TypeScript object literals should quote all eligible keys in one consistent group.
#[test]
fn test_format_object_quote_props_consistent() {
    assert_format_program!(
        r#"const x = { a: 1, "needs-quotes": 2, "default": 3 }
"#,
        r#"const x = { "a": 1, "needs-quotes": 2, "default": 3 };
"#,
        FileType::TypeScript,
        DestackFormatOptions {
            quote_props: destack_workspace::QuoteProperty::Consistent,
            ..DestackFormatOptions::default()
        }
    );
}

/// TypeScript object literals should preserve identifier-like quoted keys.
#[test]
fn test_format_object_quote_props_preserve() {
    assert_format_program!(
        r#"const x = { "normal": 1, "needs-quotes": 2, default: 3 }
"#,
        r#"const x = { "normal": 1, "needs-quotes": 2, default: 3 };
"#,
        FileType::TypeScript,
        DestackFormatOptions {
            quote_props: destack_workspace::QuoteProperty::Preserve,
            ..DestackFormatOptions::default()
        }
    );
}

/// TypeScript class fields should preserve quoted string keys.
#[test]
fn test_format_class_quote_props_consistent_without_required_quotes() {
    assert_format_program!(
        r#"class Example { "a" = 1; b = 2; }
"#,
        r#"class Example {
  "a" = 1;
  b = 2;
}
"#,
        FileType::TypeScript,
        DestackFormatOptions {
            quote_props: destack_workspace::QuoteProperty::Consistent,
            ..DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
        }
    );
}
