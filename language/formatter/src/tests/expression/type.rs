use crate::{DestackFormatOptions, assert_format_program, assert_format_program_reference_widths};
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

/// Cast-chain comment seams should stay attached across chained member continuations.
#[test]
fn test_format_type_as_comment_chain() {
    assert_format_program_reference_widths(
        r#"(activeService as unknown as QuickInputController) /* TS fail */
  .pick();
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"(activeService as unknown as QuickInputController) /* TS fail */
  .pick();
"#,
            ),
            (
                100,
                r#"(activeService as unknown as QuickInputController) /* TS fail */
  .pick();
"#,
            ),
        ],
    );
}

/// Mapped-type comment seams should stay attached to the mapped-type body.
#[test]
fn test_format_type_mapped_comment_seams() {
    assert_format_program_reference_widths(
        r#"type _Test = {
  [T in number]: T;
  // If there are any BinaryOperator members that don't have a corresponding
  // BinaryOperatorToText, then this line will error with "Type 'T' cannot
  // be used to index type 'BinaryOperatorToText'."
};
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"type _Test = {
  [T in number]: T;
  // If there are any BinaryOperator members that don't have a corresponding
  // BinaryOperatorToText, then this line will error with "Type 'T' cannot
  // be used to index type 'BinaryOperatorToText'."
};
"#,
            ),
            (
                100,
                r#"type _Test = {
  [T in number]: T;
  // If there are any BinaryOperator members that don't have a corresponding
  // BinaryOperatorToText, then this line will error with "Type 'T' cannot
  // be used to index type 'BinaryOperatorToText'."
};
"#,
            ),
        ],
    );
}

/// Leading-pipe mixed comment seams should stay attached after the separator.
#[test]
fn test_format_typescript_union_leading_pipe_mixed_comment_seams() {
    assert_format_program_reference_widths(
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
        FileType::TypeScript,
        &[
            (
                80,
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
            ),
            (
                100,
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
            ),
        ],
    );
}

/// Union head and separator comments should follow the upstream union shell exactly.
#[test]
fn test_format_typescript_union_comment_fixture() {
    assert_format_program_reference_widths(
        r#"interface _KeywordDef {
  type?: JSONType | JSONType[] // data types that keyword applies to
}

type C1 = | (
  /* 1 */ /*1*/ | (
    | (
          | A
          // A comment to force break
          | B
        )
  )
  );


type C2 = | (
  /* 1 */ /*1*/ 
  /* 1 */ | (
    | (
          | A
          // A comment to force break
          | B
        )
  )
  );
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"interface _KeywordDef {
  type?: JSONType | JSONType[]; // data types that keyword applies to
}

type C1 = /* 1 */ /*1*/
  | A
  // A comment to force break
  | B;

type C2 =
  /* 1 */ /*1*/
  /* 1 */ | A
  // A comment to force break
  | B;
"#,
            ),
            (
                100,
                r#"interface _KeywordDef {
  type?: JSONType | JSONType[]; // data types that keyword applies to
}

type C1 = /* 1 */ /*1*/
  | A
  // A comment to force break
  | B;

type C2 =
  /* 1 */ /*1*/
  /* 1 */ | A
  // A comment to force break
  | B;
"#,
            ),
        ],
    );
}

/// Leading union doc comments should stay attached to the head operand.
#[test]
fn test_format_typescript_union_leading_doc_comment() {
    assert_format_program_reference_widths(
        r#"export type AddressAllocator =
(/** Reserve a specific IP address. The pool is inferred from the address since IP pools cannot have overlapping ranges. */
| {
y: boolean
,}
| {
x: boolean }
);
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"export type AddressAllocator =
  /** Reserve a specific IP address. The pool is inferred from the address since IP pools cannot have overlapping ranges. */
  | {
      y: boolean;
    }
  | {
      x: boolean;
    };
"#,
            ),
            (
                100,
                r#"export type AddressAllocator =
  /** Reserve a specific IP address. The pool is inferred from the address since IP pools cannot have overlapping ranges. */
  | {
      y: boolean;
    }
  | {
      x: boolean;
    };
"#,
            ),
        ],
    );
}
