use crate::{DestackFormatOptions, assert_format_program};
use destack_source::FileType;

/// Conditional types with constrained infer bindings should stay inline.
#[test]
fn test_format_type_conditional_with_constrained_infer() {
    assert_format_program!(
        r#"type Result = T extends infer U extends string ? U : never
"#,
        r#"type Result = T extends infer U extends string ? U : never;
"#,
        FileType::TypeScript
    );
}

/// Mapped types with key remaps should keep the OXC-style shell.
#[test]
fn test_format_type_mapped_with_remap() {
    assert_format_program!(
        r#"type Remap = { readonly [K in keyof T as `${K}`]-?: T[K] }
"#,
        r#"type Remap = { readonly [K in keyof T as `${K}`]-?: T[K] };
"#,
        FileType::TypeScript
    );
}

/// Template literal unions should normalize to stable leading-pipe layout.
#[test]
fn test_format_type_template_literal_union_with_leading_pipe() {
    assert_format_program!(
        r#"type T = `${
  | 'W'
  | 'I'
  | 'L'
  | 'L'
  | 'B'
  | 'R'
  | 'E'
    | 'A'
  | 'K'
}${'!' | '!!'}`
"#,
        r#"type T = `${
    | 'W'
    | 'I'
    | 'L'
    | 'L'
    | 'B'
    | 'R'
    | 'E'
    | 'A'
    | 'K'}${'!' | "!!"}`;
"#,
        FileType::TypeScript
    );
}

/// Statement wrappers should preserve leading inner comment trivia.
#[test]
fn test_format_statement_cast_wrapper_preserves_leading_inner_comment() {
    assert_format_program!(
        r#"(
  // keep
  foo as Bar
)
"#,
        r#"(
  // keep
  foo as Bar
);
"#,
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Leading-pipe mixed comment seams should stay attached after the separator.
#[test]
fn test_format_typescript_union_leading_pipe_mixed_comment_seams() {
    assert_format_program!(
        r#"type A1 =
  | /**
   * 11
   */
  a
  | b

type A2 =
  | /**
   * 21
   */ a
  | b

type A3 =
  | // 31
  a
  |
  b;
"#,
        r#"type A1 =
  | /**
   * 11
   */
  a
  | b;

type A2 =
  | /**
   * 21
   */
  a
  | b;

type A3 =
  | // 31
  a
  | b;
"#,
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}
