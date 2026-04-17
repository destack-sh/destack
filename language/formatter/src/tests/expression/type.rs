use crate::{
    DestackFormatOptions, TestFormatter, assert_format_program,
    assert_format_program_reference_widths,
    assert_format_program_roundtrip_with_file_name_and_type,
};
use destack_ast::{Declaration, Expression, TypeExpression};
use destack_parser::ParserOptions;
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

/// Mapped types with key remaps should keep the shared shell.
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

/// Mapped remap block comments should stay before `as`.
#[test]
fn test_format_type_mapped_with_remap_boundary_block_comment() {
    assert_format_program!(
        r#"type Paths<T> = {
  [K in keyof T as /* remap-note */
    `get${Capitalize<K & string>}`]: () => T[K]
}
"#,
        r#"type Paths<T> = {
    [K in keyof T /* remap-note */ as `get${Capitalize<K & string>}`]: () => T[K];
};
"#,
        FileType::TypeScript
    );
}

/// Mapped remap line comments should stay on the mapped field line.
#[test]
fn test_format_type_mapped_with_remap_boundary_line_comment() {
    assert_format_program!(
        r#"type Paths<T> = {
  [K in keyof T as // remap-note
    Capitalize<K & string>]: () => T[K]
}
"#,
        r#"type Paths<T> = {
    [K in keyof T as Capitalize<K & string>]: () => T[K]; // remap-note
};
"#,
        FileType::TypeScript
    );
}

/// Already formatted remap template comments should stabilize on the shared second-pass shape.
#[test]
fn test_format_type_mapped_with_remap_boundary_line_comment_in_template_roundtrip() {
    assert_format_program_roundtrip_with_file_name_and_type(
        r#"type Paths<T> = {
    [K in keyof T as `get${Capitalize<
        K & string
    > // remap-note
    }`]: () => T[K];
};
"#,
        r#"type Paths<T> = {
    [K in keyof T as `get${Capitalize<
        K & string
    > // remap-note
    }`]: () => T[K];
};
"#,
        "main.ts",
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Formatter context should expose separator-owned remap line comments before plain remap expressions.
#[test]
fn test_format_type_mapped_plain_remap_separator_comments() {
    let (formatter, expression_id) = TestFormatter::parse_with_file_type(
        r#"type Paths<T> = {
  [K in keyof T as // remap-note
    Capitalize<K & string>]: () => T[K]
}
"#,
        FileType::TypeScript,
        |parser| parser.eat_expression(ParserOptions::default()),
    )
    .expect("parse mapped type");
    let context = formatter.context(DestackFormatOptions::default());

    let mapped_id = match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match context.tree.get(*declaration_id) {
            Declaration::Type(declaration) => declaration.value,
            other => panic!("unexpected declaration: {other:?}"),
        },
        other => panic!("unexpected expression: {other:?}"),
    };

    let key_remap = match context.tree.get(mapped_id) {
        TypeExpression::Mapped { parameter, .. } => parameter.key_remap.expect("missing key remap"),
        other => panic!("unexpected mapped value: {other:?}"),
    };

    let comments = context.raw_type_position_comments_for(key_remap);

    assert_eq!(comments.len(), 1);
    assert!(comments[0].is_line());
}

/// Formatter context should expose separator-owned remap line comments before template remap expressions.
#[test]
fn test_format_type_mapped_template_remap_separator_comments() {
    let (formatter, expression_id) = TestFormatter::parse_with_file_type(
        r#"type Paths<T> = {
  [K in keyof T as // remap-note
    `get${Capitalize<K & string>}`]: () => T[K]
}
"#,
        FileType::TypeScript,
        |parser| parser.eat_expression(ParserOptions::default()),
    )
    .expect("parse mapped type");
    let context = formatter.context(DestackFormatOptions::default());

    let mapped_id = match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match context.tree.get(*declaration_id) {
            Declaration::Type(declaration) => declaration.value,
            other => panic!("unexpected declaration: {other:?}"),
        },
        other => panic!("unexpected expression: {other:?}"),
    };

    let key_remap = match context.tree.get(mapped_id) {
        TypeExpression::Mapped { parameter, .. } => parameter.key_remap.expect("missing key remap"),
        other => panic!("unexpected mapped value: {other:?}"),
    };

    let comments = context.raw_type_position_comments_for(key_remap);

    assert_eq!(comments.len(), 1);
    assert!(comments[0].is_line());
}

/// Formatter context should expose separator-owned remap block comments before template remaps.
#[test]
fn test_format_type_mapped_template_remap_separator_block_comments() {
    let (formatter, expression_id) = TestFormatter::parse_with_file_type(
        r#"type Paths<T> = {
  [K in keyof T as /* remap-note */
    `get${Capitalize<K & string>}`]: () => T[K]
}
"#,
        FileType::TypeScript,
        |parser| parser.eat_expression(ParserOptions::default()),
    )
    .expect("parse mapped type");
    let context = formatter.context(DestackFormatOptions::default());

    let mapped_id = match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match context.tree.get(*declaration_id) {
            Declaration::Type(declaration) => declaration.value,
            other => panic!("unexpected declaration: {other:?}"),
        },
        other => panic!("unexpected expression: {other:?}"),
    };

    let key_remap = match context.tree.get(mapped_id) {
        TypeExpression::Mapped { parameter, .. } => parameter.key_remap.expect("missing key remap"),
        other => panic!("unexpected mapped value: {other:?}"),
    };

    let comments = context.raw_type_position_comments_for(key_remap);

    assert_eq!(comments.len(), 1);
    assert!(comments[0].is_block());
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

/// Mapped-type value comments should stay on the mapped field line.
#[test]
fn test_format_type_mapped_value_trailing_comment() {
    assert_format_program!(
        r#"type Flags<T> = {
  [K in keyof T]: boolean; // mapped-line
}
"#,
        r#"type Flags<T> = {
    [K in keyof T]: boolean; // mapped-line
};
"#,
        FileType::TypeScript
    );
}

/// Mapped-type value block comments should stay attached after the `:` boundary.
#[test]
fn test_format_type_mapped_value_boundary_block_comment() {
    assert_format_program!(
        r#"type Flags<T> = {
  [K in keyof T]: /* keep */ boolean;
}
"#,
        r#"type Flags<T> = {
    [K in keyof T]: /* keep */ boolean;
};
"#,
        FileType::TypeScript
    );
}

/// Mapped-type value line comments should flush after the formatted value.
#[test]
fn test_format_type_mapped_value_boundary_line_comment() {
    assert_format_program!(
        r#"type Flags<T> = {
  [K in keyof T]: // mapped-line
  boolean
}
"#,
        r#"type Flags<T> = {
    [K in keyof T]: boolean; // mapped-line
};
"#,
        FileType::TypeScript
    );
}

/// Mapped-type optional value line comments should flush after the formatted value.
#[test]
fn test_format_type_mapped_optional_value_boundary_line_comment() {
    assert_format_program!(
        r#"type Flags<T> = {
  readonly [K in keyof T]?: // map-value
  boolean
}
"#,
        r#"type Flags<T> = {
    readonly [K in keyof T]?: boolean; // map-value
};
"#,
        FileType::TypeScript
    );
}

/// Decorated single-member intersections should preserve the leading `&`.
#[test]
fn test_format_type_decorated_single_member_intersection() {
    assert_format_program!(
        r#"{
    const buffer: @addrspace("shared") &Buffer = value;
    function build(value: Buffer): @addrspace("shared") &Buffer {
        return value;
    }
}
"#,
        r#"{
    const buffer: @addrspace("shared") &Buffer = value;
    function build(value: Buffer): @addrspace("shared") &Buffer {
        return value;
    }
}
"#,
        FileType::TypeScript
    );
}

/// Leading-pipe mixed comments should stay attached after the separator.
#[test]
fn test_format_union_leading_pipe_mixed_comments() {
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

/// Already formatted union wrapper comments should stay stable on the second pass.
#[test]
fn test_format_union_comment_layout_roundtrip() {
    assert_format_program_roundtrip_with_file_name_and_type(
        r#"interface _KeywordDef {
  type?: JSONType | JSONType[]; // data types that keyword applies to
}

type C1 =
  /* 1 */ /*1*/
  | A
  // A comment to force break
  | B;

type C2 =
  /* 1 */ /*1*/
  /* 1 */ | A
  // A comment to force break
  | B;
"#,
        r#"interface _KeywordDef {
  type?: JSONType | JSONType[]; // data types that keyword applies to
}

type C1 =
  /* 1 */ /*1*/
  | A
  // A comment to force break
  | B;

type C2 =
  /* 1 */ /*1*/
  /* 1 */ | A
  // A comment to force break
  | B;
"#,
        "main.ts",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}

/// Leading union doc comments should stay attached to the head operand.
#[test]
fn test_format_union_leading_doc_comment() {
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
fn test_format_conditional_nested_test_layout() {
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
fn test_format_union_parenthesis_layout() {
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
fn test_format_single_member_union_layout() {
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

/// Parenthesized unions should keep the last arm line comment on the last arm.
#[test]
fn test_format_parenthesized_union_last_arm_comment() {
    assert_format_program!(
        r#"type Result = (
  | "a" // arm-a
  | "b" // arm-b
)[] // final-tail
"#,
        r#"type Result = (
    | "a" // arm-a
    | "b" // arm-b
)[]; // final-tail
"#,
        FileType::TypeScript
    );
}

/// Union doc heads should collapse inline at wider widths like the reference formatter.
#[test]
fn test_format_union_doc_head_width_behavior() {
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
fn test_format_union_annotation_width_behavior() {
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
fn test_format_template_literal_union_width_behavior() {
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
fn test_format_template_literal_conditional_width_behavior() {
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
fn test_format_type_assertion_assignment_layout() {
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

/// Union comments inside `as` assertions and type arguments should preserve layout.
#[test]
fn test_format_union_type_argument_and_as_assertion_comments() {
    assert_format_program_reference_widths(
        r#"// generic argument list
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


// as
0 as
  // comment
  string | number | undefined;

console.log(
  0 as
    // comment
    string | number | undefined
);
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// generic argument list
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

// as
0 as
// comment
string | number | undefined;

console.log(
  0 as
  // comment
  string | number | undefined,
);
"#,
            ),
            (
                100,
                r#"// generic argument list
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

// as
0 as
// comment
string | number | undefined;

console.log(
  0 as
  // comment
  string | number | undefined,
);
"#,
            ),
        ],
    );
}

/// Arrow functions in module files should preserve trailing type-parameter commas.
#[test]
fn test_format_module_arrow_type_parameter_trailing_comma() {
    // module extension, 80
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

    // module extension, 100
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

    // common module extension, 80
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

    // common module extension, 100
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

/// Single constrained lambda generic parameters should not force trailing commas.
#[test]
fn test_format_module_arrow_type_parameter_constraint_without_trailing_comma() {
    assert_format_program_roundtrip_with_file_name_and_type(
        r#"// index.mts
const fn = <T extends string>() => {}
"#,
        r#"// index.mts
const fn = <T extends string>() => {};
"#,
        "index.mts",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}
