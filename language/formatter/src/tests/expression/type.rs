use crate::{
    DestackFormatOptions, assert_format_program, assert_format_program_reference_widths,
    assert_format_program_roundtrip_with_file_name_and_type,
};
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
        r#"type T = `${"W" | "I" | "L" | "L" | "B" | "R" | "E" | "A" | "K"}${"!" | "!!"}`;
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

/// Cast-chain comments should stay attached across chained member continuations.
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

/// Multiline block comments before cast types should stay with the type side.
#[test]
fn test_format_type_as_multiline_block_comment_before_type() {
    assert_format_program!(
        r#"foo as /*
 * keep
 */ Bar
"#,
        r#"foo as /*
 * keep
 */ Bar;
"#,
        FileType::TypeScript
    );
}

/// Mapped-type comments should stay attached to the mapped-type body.
#[test]
fn test_format_type_mapped_comments() {
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

/// Leading-pipe mixed comments should stay attached after the separator.
#[test]
fn test_format_typescript_union_leading_pipe_mixed_comments() {
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
     */ a
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
     */ a
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

/// Union head and separator comments should follow the expected union shell exactly.
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

/// Nested conditional types should follow the reference break behavior.
#[test]
fn test_format_typescript_conditional_nested_test_layout() {
    assert_format_program_reference_widths(
        r#"type IsUnion<T> = (
  Testtttttttttttttttttttttttttttttttttt extends any ? false : never
) extends false
  ? false
  : true



export const IsUnionType = (
  Testtttttttttttttttttttttttttttttttttt ? false : never
)  ? false
  : true
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"type IsUnion<T> = (
  Testtttttttttttttttttttttttttttttttttt extends any ? false : never
) extends false
  ? false
  : true;

export const IsUnionType = (
  Testtttttttttttttttttttttttttttttttttt ? false : never
)
  ? false
  : true;
"#,
            ),
            (
                100,
                r#"type IsUnion<T> = (Testtttttttttttttttttttttttttttttttttt extends any ? false : never) extends false
  ? false
  : true;

export const IsUnionType = (Testtttttttttttttttttttttttttttttttttt ? false : never) ? false : true;
"#,
            ),
        ],
    );
}

/// Leading-pipe unions should drop redundant grouping around single arms.
#[test]
fn test_format_typescript_union_parenthesis_layout() {
    assert_format_program_reference_widths(
        r#"type T1<B> = | (B extends any ? number : string);
type T2 = | (() => void);
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"type T1<B> = B extends any ? number : string;
type T2 = () => void;
"#,
            ),
            (
                100,
                r#"type T1<B> = B extends any ? number : string;
type T2 = () => void;
"#,
            ),
        ],
    );
}

/// Single-member unions should not keep redundant parentheses.
#[test]
fn test_format_typescript_single_member_union_layout() {
    assert_format_program_reference_widths(
        r#"// Single-member unions should not have unnecessary parentheses
type Items = ( | number)[];
type Items2 = ( & number)[];

// Multi-member unions should keep parentheses
type Items3 = (string | number)[];

// Simple case without array
type Simple = | number;
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// Single-member unions should not have unnecessary parentheses
type Items = number[];
type Items2 = number[];

// Multi-member unions should keep parentheses
type Items3 = (string | number)[];

// Simple case without array
type Simple = number;
"#,
            ),
            (
                100,
                r#"// Single-member unions should not have unnecessary parentheses
type Items = number[];
type Items2 = number[];

// Multi-member unions should keep parentheses
type Items3 = (string | number)[];

// Simple case without array
type Simple = number;
"#,
            ),
        ],
    );
}

/// Union doc heads should collapse inline at wider widths like the reference formatter.
#[test]
fn test_format_typescript_union_doc_head_width_behavior() {
    assert_format_program_reference_widths(
        r#"export type xxxxxxxxxxxxxx =
  /** xxxx
   */
  | { xxxxxxxxxxxxxxx: true }
  | { xxxxxxxxxxxxxxx: false; xxxxxxxxxxxxxxx: bigint | null };
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"export type xxxxxxxxxxxxxx =
  /** xxxx
   */
  | { xxxxxxxxxxxxxxx: true }
  | { xxxxxxxxxxxxxxx: false; xxxxxxxxxxxxxxx: bigint | null };
"#,
            ),
            (
                100,
                r#"export type xxxxxxxxxxxxxx =
  /** xxxx
   */
  { xxxxxxxxxxxxxxx: true } | { xxxxxxxxxxxxxxx: false; xxxxxxxxxxxxxxx: bigint | null };
"#,
            ),
        ],
    );
}

/// Union annotations in type positions should keep reference width behavior.
#[test]
fn test_format_typescript_union_annotation_width_behavior() {
    assert_format_program_reference_widths(
        r#"export default class TestUnionTypeAnnotation1 {
  private prop!: /* comment */
    LongLongLongLongLongLongType[] | LongLongLongLongLongLongType[];

  private accessor prop2!: /* comment */
    LongLongLongLongLongLongType[] | LongLongLongLongLongLongType[];
}

export interface TestUnionTypeAnnotation2 {
  property: /* comment */
    LongLongLongLongLongLongType[] | LongLongLongLongLongLongType[];
}
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"export default class TestUnionTypeAnnotation1 {
  private prop!: /* comment */
    LongLongLongLongLongLongType[] | LongLongLongLongLongLongType[];

  private accessor prop2!: /* comment */
    LongLongLongLongLongLongType[] | LongLongLongLongLongLongType[];
}

export interface TestUnionTypeAnnotation2 {
  property: /* comment */
    LongLongLongLongLongLongType[] | LongLongLongLongLongLongType[];
}
"#,
            ),
            (
                100,
                r#"export default class TestUnionTypeAnnotation1 {
  private prop!: /* comment */ LongLongLongLongLongLongType[] | LongLongLongLongLongLongType[];

  private accessor prop2!: /* comment */
    LongLongLongLongLongLongType[] | LongLongLongLongLongLongType[];
}

export interface TestUnionTypeAnnotation2 {
  property: /* comment */ LongLongLongLongLongLongType[] | LongLongLongLongLongLongType[];
}
"#,
            ),
        ],
    );
}

/// Template literal unions should collapse inline at wider widths like the reference formatter.
#[test]
fn test_format_typescript_template_literal_union_width_behavior() {
    assert_format_program_reference_widths(
        r#"export type T = `${
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
        FileType::TypeScript,
        &[
            (
                80,
                r#"export type T = `${
  | "W"
  | "I"
  | "L"
  | "L"
  | "B"
  | "R"
  | "E"
  | "A"
  | "K"}${"!" | "!!"}`;
"#,
            ),
            (
                100,
                r#"export type T = `${"W" | "I" | "L" | "L" | "B" | "R" | "E" | "A" | "K"}${"!" | "!!"}`;
"#,
            ),
        ],
    );
}

/// Template literal conditionals should follow the reference width behavior.
#[test]
fn test_format_typescript_template_literal_conditional_width_behavior() {
    assert_format_program_reference_widths(
        r#"type templateLiteralType = `${
  TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? '_'
    : ''
}`;

type CamelToSnakeCase<TCamelCaseString extends string> =
  TCamelCaseString extends `${infer TStringConvertedSoFar}${infer TStringYetToConvert}`
    ? `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
        ? '_'
        : ''}${Lowercase<TStringConvertedSoFar>}${CamelToSnakeCase<TStringYetToConvert>}`
    : TCamelCaseString;
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"type templateLiteralType =
  `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""}`;

type CamelToSnakeCase<TCamelCaseString extends string> =
  TCamelCaseString extends `${infer TStringConvertedSoFar}${infer TStringYetToConvert}`
    ? `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
        ? "_"
        : ""}${Lowercase<TStringConvertedSoFar>}${CamelToSnakeCase<TStringYetToConvert>}`
    : TCamelCaseString;
"#,
            ),
            (
                100,
                r#"type templateLiteralType = `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
  ? "_"
  : ""}`;

type CamelToSnakeCase<TCamelCaseString extends string> =
  TCamelCaseString extends `${infer TStringConvertedSoFar}${infer TStringYetToConvert}`
    ? `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
        ? "_"
        : ""}${Lowercase<TStringConvertedSoFar>}${CamelToSnakeCase<TStringYetToConvert>}`
    : TCamelCaseString;
"#,
            ),
        ],
    );
}

/// Type assertions and satisfies expressions should match the reference assignment layout.
#[test]
fn test_format_typescript_type_assertion_assignment_layout() {
    assert_format_program_reference_widths(
        r#"(type) as unknown;
(type) satisfies unknown;

() => (type) as unknown;
() => (type) satisfies unknown;

((type) as any)['t'];
((type) satisfies any)['t'];
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"(type) as unknown;
(type) satisfies unknown;

() => type as unknown;
() => type satisfies unknown;

(type as any)["t"];
(type satisfies any)["t"];
"#,
            ),
            (
                100,
                r#"(type) as unknown;
(type) satisfies unknown;

() => type as unknown;
() => type satisfies unknown;

(type as any)["t"];
(type satisfies any)["t"];
"#,
            ),
        ],
    );
}

/// Union comments inside type assertions and type arguments should preserve layout.
#[test]
fn test_format_typescript_union_type_argument_and_assertion_comments() {
    assert_format_program_reference_widths(
        r#"// TSTypeParameterInstantiation
export class ClassTest extends Modal<
  // comment
  string | number | undefined
> {
}

Math.random<
  // comment
  string | number | undefined
>;

Math.random<
  // comment
  string | number | undefined
>();


// TypeAssertion
<
  // comment
  string | number | undefined
>0;

console.log(
  <
    // comment
    string | number | undefined
  >0
);
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// TSTypeParameterInstantiation
export class ClassTest extends Modal<
  // comment
  string | number | undefined
> {}

Math.random<
  // comment
  string | number | undefined
>;

Math.random<
  // comment
  string | number | undefined
>();

// TypeAssertion
<
  // comment
  string | number | undefined
>0;

console.log(
  <
    // comment
    string | number | undefined
  >0,
);
"#,
            ),
            (
                100,
                r#"// TSTypeParameterInstantiation
export class ClassTest extends Modal<
  // comment
  string | number | undefined
> {}

Math.random<
  // comment
  string | number | undefined
>;

Math.random<
  // comment
  string | number | undefined
>();

// TypeAssertion
<
  // comment
  string | number | undefined
>0;

console.log(
  <
    // comment
    string | number | undefined
  >0,
);
"#,
            ),
        ],
    );
}

/// Module TypeScript arrow functions should preserve trailing type-parameter commas.
#[test]
fn test_format_typescript_module_arrow_type_parameter_trailing_comma() {
    // mts, 80
    assert_format_program_roundtrip_with_file_name_and_type(
        r#"// index.mts
const fn = <T,>() => {}
"#,
        r#"// index.mts
const fn = <T,>() => {};
"#,
        "index.mts",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );

    // mts, 100
    assert_format_program_roundtrip_with_file_name_and_type(
        r#"// index.mts
const fn = <T,>() => {}
"#,
        r#"// index.mts
const fn = <T,>() => {};
"#,
        "index.mts",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100).with_indent_width(2),
    );

    // cts, 80
    assert_format_program_roundtrip_with_file_name_and_type(
        r#"// index.cts
const fn = <T,>() => {}
"#,
        r#"// index.cts
const fn = <T,>() => {};
"#,
        "index.cts",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );

    // cts, 100
    assert_format_program_roundtrip_with_file_name_and_type(
        r#"// index.cts
const fn = <T,>() => {}
"#,
        r#"// index.cts
const fn = <T,>() => {};
"#,
        "index.cts",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100).with_indent_width(2),
    );
}
