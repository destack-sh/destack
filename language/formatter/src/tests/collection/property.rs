use crate::{DestackFormatOptions, assert_format, assert_format_program};
use destack_ast::DeclarationDescriptor;
use destack_source::{FileType, LanguageType};

/// Empty structs should collapse cleanly.
#[test]
fn test_format_struct_empty() {
    assert_format!(
        r#"struct Foo { }"#,
        r#"struct Foo {}"#,
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
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
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
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
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
        DestackFormatOptions {
            language_type: LanguageType::TypeScript,
            ..DestackFormatOptions::default_tab()
        }
    );
}

/// Quoted constructor parameter properties should expand into one parameter per line.
#[test]
fn test_format_typescript_constructor_parameter_properties_expand_for_quoted_constructor_name() {
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
