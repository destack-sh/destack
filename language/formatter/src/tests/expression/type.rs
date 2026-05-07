use crate::{
    DestackFormatOptions, assert_format_program, assert_format_program_reference_widths,
    assert_format_program_roundtrip_with_file_name_and_type,
    assert_format_program_roundtrip_with_file_type,
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

/// Conditional type trailing comments should stay on the branch they trail.
#[test]
fn test_format_type_conditional_trailing_branch_comments() {
    assert_format_program!(
        r#"type Awaited<T> = T extends null | undefined
  ? T // special case for null
  : T extends object
    ? F extends (value: infer V) => any // if callable
      ? Awaited<V> // recursively unwrap
      : never // not callable
    : T; // non-object
"#,
        r#"type Awaited<T> = T extends null | undefined
    ? T // special case for null
    : T extends object
      ? F extends (value: infer V) => any // if callable
          ? Awaited<V> // recursively unwrap
          : never // not callable
      : T; // non-object
"#,
        FileType::TypeScriptDeclaration
    );
}

/// Conditional type alternate comments should stay on the `:` branch.
#[test]
fn test_format_type_conditional_alternate_line_comment() {
    assert_format_program!(
        r#"type A = typeof globalThis extends {
    onmessage: any;
    ReportingObserver: any;
    CompressionStream: infer T;
} ? T
    // TS 4.8, 4.9, 5.0
    : typeof globalThis extends { onmessage: any; TransformStream: { prototype: infer T } } ? {
            prototype: T;
            new(format: "deflate" | "deflate-raw" | "gzip"): T;
        }
    : typeof import("stream/web").CompressionStream;
"#,
        r#"type A = typeof globalThis extends {
    onmessage: any;
    ReportingObserver: any;
    CompressionStream: infer T;
}
    ? T
    : // TS 4.8, 4.9, 5.0
      typeof globalThis extends { onmessage: any; TransformStream: { prototype: infer T } }
      ? {
            prototype: T;
            new (format: "deflate" | "deflate-raw" | "gzip"): T;
        }
      : typeof import("stream/web").CompressionStream;
"#,
        FileType::TypeScriptDeclaration
    );
}

/// Empty object type literals should stay compact.
#[test]
fn test_format_type_empty_object_literal() {
    assert_format_program!(
        r#"type A = typeof globalThis extends { onmessage: any } ? {} : AbortController
type B = T | {}
"#,
        r#"type A = typeof globalThis extends { onmessage: any } ? {} : AbortController;
type B = T | {};
"#,
        FileType::TypeScript
    );
}

/// Construct signatures should keep a space before parameters.
#[test]
fn test_format_type_construct_signature_spacing() {
    assert_format_program!(
        r#"type B = { new(): Foo; new(...args: any): Bar }
type C = F extends abstract new(...args: any) => infer T ? T : never
"#,
        r#"type B = { new (): Foo; new (...args: any): Bar };
type C = F extends abstract new (...args: any) => infer T ? T : never;
"#,
        FileType::TypeScript
    );
}

/// TypeScript generic const parameters should keep their source keyword.
#[test]
fn test_format_type_const_generic_parameter() {
    assert_format_program!(
        r#"type Fn = <const T>(value: T) => T
"#,
        r#"type Fn = <const T>(value: T) => T;
"#,
        FileType::TypeScript
    );
}

/// TypeScript labeled tuple rest elements should use rest-first spelling.
#[test]
fn test_format_type_labeled_tuple_rest() {
    assert_format_program!(
        r#"type AnyRest = [...args: any[]]
"#,
        r#"type AnyRest = [...args: any[]];
"#,
        FileType::TypeScript
    );
}

/// Type member doc comments should stay before the member, not before generated terminators.
#[test]
fn test_format_type_member_doc_comment_after_missing_terminator() {
    assert_format_program!(
        r#"interface WebidlErrors {
  /**
   * @description Instantiate an error
   */
  exception (opts: { header: string, message: string }): TypeError
  /**
   * @description Instantiate an error when conversion from one type to another has failed
   */
  conversionFailed (opts: { prefix: string, argument: string, types: string[] }): TypeError
}
"#,
        r#"interface WebidlErrors {
    /**
     * @description Instantiate an error
     */
    exception(opts: { header: string; message: string }): TypeError;
    /**
     * @description Instantiate an error when conversion from one type to another has failed
     */
    conversionFailed(opts: { prefix: string; argument: string; types: string[] }): TypeError;
}
"#,
        FileType::TypeScriptDeclaration
    );
}

/// Comment-only type members should stay visible before the closing brace.
#[test]
fn test_format_type_member_comment_only_tail() {
    assert_format_program!(
        r#"interface BlobPropertyBag {
  /** Set a default "type". Not yet implemented. */
  type?: string;
  /** Not implemented in Bun yet. */
  // endings?: "transparent" | "native";
}
"#,
        r#"interface BlobPropertyBag {
    /** Set a default "type". Not yet implemented. */
    type?: string;
    /** Not implemented in Bun yet. */
    // endings?: "transparent" | "native";
}
"#,
        FileType::TypeScriptDeclaration
    );
}

/// Type member doc comments should preserve source blank lines.
#[test]
fn test_format_type_member_blank_line_before_doc_comment() {
    assert_format_program!(
        r#"interface A {
  method(options: {
    source: string;

    /**
     * Library names to link against
     */
    library?: string[] | string;
  }): void;
}
"#,
        r#"interface A {
    method(options: {
        source: string;

        /**
         * Library names to link against
         */
        library?: string[] | string;
    }): void;
}
"#,
        FileType::TypeScriptDeclaration
    );
}

/// Readonly array types should not capture surrounding union arms.
#[test]
fn test_format_type_readonly_array_union() {
    assert_format_program!(
        r#"type Args = readonly string[] | undefined | null
"#,
        r#"type Args = readonly string[] | undefined | null;
"#,
        FileType::TypeScript
    );
}

/// Slice types should keep bracket form.
#[test]
fn test_format_type_slice() {
    assert_format_program!(
        r#"type Values = [Value]
"#,
        r#"type Values = [Value];
"#,
        FileType::Destack
    );
}

/// Slice element types keep readonly prefixes inside brackets.
#[test]
fn test_format_type_readonly_slice() {
    assert_format_program!(
        r#"type Values = [readonly Value]
"#,
        r#"type Values = [readonly Value];
"#,
        FileType::Destack
    );
}

/// Fixed array types should keep their length expression.
#[test]
fn test_format_type_fixed_array() {
    assert_format_program!(
        r#"type Bytes = [byte; 32]
"#,
        r#"type Bytes = [byte; 32];
"#,
        FileType::Destack
    );
}

/// Class method trailing comments should stay with the terminated member.
#[test]
fn test_format_class_method_trailing_line_comment() {
    assert_format_program!(
        r#"declare namespace cluster {
  export class Worker extends EventEmitter {
  /**
   * events.EventEmitter
   *   1. disconnect
   *   2. error
   *   3. exit
   *   4. listening
   *   5. message
   *   6. online
   */
  addListener(event: string, listener: (...args: any[]) => void): this;
  addListener(event: "disconnect", listener: () => void): this;
  addListener(event: "error", listener: (error: Error) => void): this;
  addListener(event: "exit", listener: (code: number, signal: string) => void): this;
  addListener(event: "listening", listener: (address: Address) => void): this;
  addListener(event: "message", listener: (message: any, handle: net.Socket | net.Server) => void): this; // the handle is a net.Socket or net.Server object, or undefined.
  addListener(event: "online", listener: () => void): this;
  on(event: string, listener: (...args: any[]) => void): this;
  on(event: "message", listener: (message: any, handle: net.Socket | net.Server) => void): this; // the handle is a net.Socket or net.Server object, or undefined.
  on(event: "online", listener: () => void): this;
  }
}
"#,
        r#"declare namespace cluster {
    export class Worker extends EventEmitter {
        /**
         * events.EventEmitter
         *   1. disconnect
         *   2. error
         *   3. exit
         *   4. listening
         *   5. message
         *   6. online
         */
        addListener(event: string, listener: (...args: any[]) => void): this;
        addListener(event: "disconnect", listener: () => void): this;
        addListener(event: "error", listener: (error: Error) => void): this;
        addListener(event: "exit", listener: (code: number, signal: string) => void): this;
        addListener(event: "listening", listener: (address: Address) => void): this;
        addListener(
            event: "message",
            listener: (message: any, handle: net.Socket | net.Server) => void,
        ): this; // the handle is a net.Socket or net.Server object, or undefined.
        addListener(event: "online", listener: () => void): this;
        on(event: string, listener: (...args: any[]) => void): this;
        on(
            event: "message",
            listener: (message: any, handle: net.Socket | net.Server) => void,
        ): this; // the handle is a net.Socket or net.Server object, or undefined.
        on(event: "online", listener: () => void): this;
    }
}
"#,
        FileType::TypeScript
    );
}

/// Optional computed class methods should keep the optional marker before generics.
#[test]
fn test_format_class_method_optional_computed_key() {
    assert_format_program!(
        r#"class EventEmitter<T> {
  [EventEmitter.captureRejectionSymbol]?<K>(error: Error, event: Key<K, T>, ...args: Args<K, T>): void;
}
"#,
        r#"class EventEmitter<T> {
    [EventEmitter.captureRejectionSymbol]?<K>(
        error: Error,
        event: Key<K, T>,
        ...args: Args<K, T>
    ): void;
}
"#,
        FileType::TypeScript
    );
}

/// Function-type return annotations should use arrow-function parenthesis rules.
#[test]
fn test_format_function_type_return_parentheses() {
    assert_format_program!(
        r#"function f(): (value: Value) => void {}
const g = (): (value: Value) => void => (value: Value) => {};
"#,
        r#"function f(): (value: Value) => void {}
const g = (): ((value: Value) => void) => (value: Value) => {};
"#,
        FileType::TypeScript
    );
}

/// Mapped types with key remaps should keep the shared layout.
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
fn test_format_type_mapped_with_remap_separator_block_comment() {
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
fn test_format_type_mapped_with_remap_separator_line_comment() {
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
fn test_format_type_mapped_with_remap_separator_line_comment_in_template_roundtrip() {
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

/// Statement cast comments should stay attached without preserving wrapper nodes.
#[test]
fn test_format_statement_cast_keeps_leading_comment_without_wrapper_node() {
    assert_format_program!(
        r#"(
  // keep
  foo as Bar
)
"#,
        r#"// keep
foo as Bar;
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

/// Inline block comments in type arguments should keep separator spacing.
#[test]
fn test_format_type_argument_inline_block_comment_spacing() {
    assert_format_program!(
        r#"type T = Foo</*a*/ string>
type U = Foo<string, /*b*/ number>
"#,
        r#"type T = Foo</*a*/ string>;
type U = Foo<string, /*b*/ number>;
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

/// Mapped-type own-line comments after `{` should stay before the key head.
#[test]
fn test_format_type_mapped_leading_body_line_comment() {
    assert_format_program!(
        r#"type Flags<T> = {
  // map-head
  [K in keyof T]: boolean
}
"#,
        r#"type Flags<T> = {
    // map-head
    [K in keyof T]: boolean;
};
"#,
        FileType::TypeScript
    );
}

/// Mapped-type block comments after `{` should stay before the key head.
#[test]
fn test_format_type_mapped_leading_body_block_comment() {
    assert_format_program!(
        r#"type Flags<T> = {
  /* map-head */
  [K in keyof T]: boolean
}
"#,
        r#"type Flags<T> = {
    /* map-head */
    [K in keyof T]: boolean;
};
"#,
        FileType::TypeScript
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

/// Mapped-type value block comments should stay attached after the `:` separator.
#[test]
fn test_format_type_mapped_value_separator_block_comment() {
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

/// Mapped-type value block comments after semicolons should normalize before the semicolon.
#[test]
fn test_format_type_mapped_value_trailing_block_comment_after_semicolon() {
    assert_format_program!(
        r#"type Flags<T> = {
  [K in keyof T]: boolean; /* mapped-block */
}
"#,
        r#"type Flags<T> = {
    [K in keyof T]: boolean /* mapped-block */;
};
"#,
        FileType::TypeScript
    );
}

/// Mapped-type value line comments should flush after the formatted value.
#[test]
fn test_format_type_mapped_value_separator_line_comment() {
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
fn test_format_type_mapped_optional_value_separator_line_comment() {
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
        FileType::Destack
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

/// Leading-separator doc comments should be visible as first-arm prefix comments.
#[test]
fn test_format_union_leading_pipe_multiline_doc_comment_reaches_first_arm_prefix() {
    assert_format_program_roundtrip_with_file_type(
        r#"type A =
  | /**
   * 11
   */
  a
  | b
"#,
        r#"type A =
  | /**
     * 11
     */
    a
  | b;
"#,
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
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

/// Union doc heads should collapse inline at wider widths.
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

/// Template literal unions should collapse inline at wider widths.
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
