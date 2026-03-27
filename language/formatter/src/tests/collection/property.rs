use crate::{
    DestackFormatOptions, TestFormatter, assert_format,
    assert_format_program_idempotent_with_file_type,
    assert_format_program_roundtrip_with_file_type,
};
use destack_ast::DeclarationDescriptor;
use destack_source::{FileType, LanguageType};

#[test]
fn test_format_struct_empty() {
    assert_format!(
        "struct Foo { }",
        "struct Foo {}",
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_struct_with_fields() {
    assert_format!(
        "struct Foo { a: int32; b: boolean }",
        "struct Foo {\n\ta: int32;\n\tb: boolean;\n}",
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_struct_with_modified_fields() {
    assert_format!(
        "struct Foo { readonly a: int32; private b: boolean }",
        "struct Foo {\n\treadonly a: int32;\n\tprivate b: boolean;\n}",
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_struct_with_name() {
    assert_format!(
        "struct Foo { a: int32 }",
        "struct Foo {\n\ta: int32;\n}",
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_struct_with_fields_and_defaults() {
    assert_format!(
        "struct Foo { a?: int32 = 42; b: boolean }",
        "struct Foo {\n\ta?: int32 = 42;\n\tb: boolean;\n}",
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_class_with_abstract_override_field() {
    assert_format!(
        "class Foo { abstract override bar: int32 }",
        "class Foo {\n\tabstract override bar: int32;\n}",
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
        DestackFormatOptions {
            language_type: LanguageType::TypeScript,
            ..DestackFormatOptions::default_tab()
        }
    );
}

#[test]
fn test_format_struct_with_static_parameters_and_inheritance() {
    assert_format!(
        "struct Foo<T: Numeric> extends Bar implements Baz { }",
        "struct Foo<T: Numeric> extends Bar implements Baz {}",
        |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_typescript_constructor_parameter_properties_expand_for_quoted_constructor_name() {
    let source = r#"
[
  class {
    "constructor"(protected x: number, private y: string) {}
  },
]
"#;
    let expected = r#"
[
  class {
    constructor(
      protected x: number,
      private y: string,
    ) {}
  },
];
"#;
    assert_format_program_roundtrip_with_file_type(
        source,
        expected.trim_start(),
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}

#[test]
fn test_format_typescript_field_initializer_call_breaks_after_equals_stably() {
    let source = r#"
class X {
  value: Type = memoize((arg: Arg): Ret => { const result = doSomething(arg); return result; });
}
"#;
    let expected = r#"
class X {
  value: Type =
    memoize((arg: Arg): Ret => {
      const result = doSomething(arg);
      return result;
    });
}
"#;
    assert_format_program_roundtrip_with_file_type(
        source,
        expected.trim_start(),
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}

#[test]
fn test_format_typescript_field_initializer_with_type_arguments_keeps_equals_inline() {
    let source = r#"
export class Test {
  readonly coordinates = model.required<
    Immutable<{
      latitude: number;
      longitude: number;
    }>
  >();
}
"#;
    let expected = r#"
export class Test {
  readonly coordinates = model.required<
    Immutable<{
      latitude: number;
      longitude: number;
    }>
  >();
}
"#;
    assert_format_program_roundtrip_with_file_type(
        source,
        expected.trim_start(),
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}

#[test]
fn test_format_destack_method_keeps_explicit_this_parameter_roundtrip() {
    let source = r#"
extension<T> for Slice<T> {
  indexSet(this: &Slice<T>, i: number, value: T): void {
    undefined!;
  }
}
"#;
    let expected = r#"
extension<T> for Slice<T> {
  indexSet(this: &Slice<T>, i: number, value: T): void {
    undefined!;
  }
}
"#;
    assert_format_program_roundtrip_with_file_type(
        source,
        expected.trim_start(),
        FileType::Destack,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}

#[test]
fn test_format_destack_method_keeps_explicit_this_parameter_idempotent() {
    let source = r#"
extension<T> for Slice<T> {
  indexSet(this: &Slice<T>, i: number, value: T): void {
    undefined!;
  }

  reverse(this: &Slice<T>): void {
    undefined!;
  }
}
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::Destack,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}
