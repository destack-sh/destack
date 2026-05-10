use crate::{
    DestackFormatOptions, assert_format, assert_format_program,
    assert_format_program_reference_widths,
};
use destack_source::FileType;

#[test]
fn test_format_enum_empty() {
    assert_format!(
        "enum { }",
        "enum {}",
        |p| p.parse_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_enum_with_simple_fields() {
    assert_format!(
        "enum { A, B }",
        r#"enum {
	A,
	B,
}"#,
        |p| p.parse_expression(),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_enum_with_annotations() {
    assert_format_program!(
        r#"@description("The status of a task.") enum Status { @default Todo; Done }"#,
        r#"@description("The status of a task.") enum Status {
    @default
    Todo,
    Done,
}
"#,
        FileType::Destack,
    );
}

#[test]
fn test_format_enum_with_generic_parameters() {
    let source = r"enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    A = 1,
    B = T,
    @if(IsSomething)
    C = 3,
}";
    assert_format!(
        source,
        source,
        |p| p.parse_expression(),
        DestackFormatOptions::default()
    );
}

/// Enum member blank lines before doc comments should follow source spacing.
#[test]
fn test_format_enum_member_blank_lines() {
    assert_format_program!(
        r#"enum FFIType {
  /**
   * 8-bit signed integer
   */
  i8 = 1,

  /**
   * 8-bit unsigned integer
   */
  u8 = 2,
}
"#,
        r#"enum FFIType {
    /** 8-bit signed integer */
    i8 = 1,

    /** 8-bit unsigned integer */
    u8 = 2,
}
"#,
        FileType::TypeScriptDeclaration,
    );
}

/// Class extends sequence expressions should keep required parentheses.
#[test]
fn test_format_class_extends_sequence_expression_parentheses() {
    assert_format_program!(
        r#"class A extends (a, b) {}
"#,
        r#"class A extends (a, b) {}
"#,
        FileType::TypeScript,
    );
}

/// Class member comments after an extends clause should stay on their members.
#[test]
fn test_format_class_extends_member_comments() {
    assert_format_program!(
        r#"export class Client extends Dispatcher {
  constructor(url: string | URL, options?: Client.Options);
  /** Property to get and set the pipelining factor. */
  pipelining: number;
  /** `true` after `client.close()` has been called. */
  closed: boolean;
}
"#,
        r#"export class Client extends Dispatcher {
    constructor(url: string | URL, options?: Client.Options);
    /** Property to get and set the pipelining factor. */
    pipelining: number;
    /** `true` after `client.close()` has been called. */
    closed: boolean;
}
"#,
        FileType::TypeScriptDeclaration,
    );
}

/// Empty namespace body comments should stay as dangling body comments.
#[test]
fn test_format_namespace_empty_body_comment() {
    assert_format_program!(
        r#"declare namespace Intl {
  // Empty
}
"#,
        r#"declare namespace Intl {
    // Empty
}
"#,
        FileType::TypeScriptDeclaration,
    );
}

/// Namespace body tail comments should stay before the closing brace.
#[test]
fn test_format_namespace_tail_comment() {
    assert_format_program!(
        r#"declare namespace RedisClient {
  type KeyLike = string;
  type StringPubSubListener = (message: string, channel: string) => void;

  // Buffer subscriptions are not yet implemented
  // type BufferPubSubListener = (message: Uint8Array<ArrayBuffer>, channel: string) => void;
}
"#,
        r#"declare namespace RedisClient {
    type KeyLike = string;
    type StringPubSubListener = (message: string, channel: string) => void;

    // Buffer subscriptions are not yet implemented
    // type BufferPubSubListener = (message: Uint8Array<ArrayBuffer>, channel: string) => void;
}
"#,
        FileType::TypeScriptDeclaration,
    );
}

/// Ignored members should keep leading documentation outside the raw ignored range.
#[test]
fn test_format_ignored_member_leading_doc_comment() {
    assert_format_program!(
        r#"interface RedisClient {
  /**
   * Get hash field values with expiration options
   */
  //prettier-ignore
  hgetex(key: KeyLike, fieldsKeyword: "FIELDS", numfields: number, ...fields: KeyLike[]): Promise<Array<string | null>>;
}
"#,
        r#"interface RedisClient {
    /** Get hash field values with expiration options */
    //prettier-ignore
    hgetex(key: KeyLike, fieldsKeyword: "FIELDS", numfields: number, ...fields: KeyLike[]): Promise<Array<string | null>>;
}
"#,
        FileType::TypeScriptDeclaration,
    );
}

/// Class type layout should keep the expected extends and implements forms.
#[test]
fn test_format_class_type_layout() {
    assert_format_program_reference_widths(
        r#"// Accessor with type annotation
class Typed {
  accessor foo: string = "bar";
  accessor veryLongPropertyName: SomeReallyLongTypeName = someReallyLongFunctionName(argumentOne, argumentTwo);
}

// Accessor with modifiers
class Modifiers {
  public accessor foo: string = "bar";
  private accessor bar: number = 42;
  protected accessor baz: boolean = true;
  static accessor qux: string = "static";
  override accessor overridden: string = "overridden";
}

// Abstract accessor
abstract class AbstractAccessor {
  abstract accessor foo: string;
}

// Accessor with definite assignment
class Definite {
  accessor foo!: string;
}

// Accessor with long type annotation and value
class LongAnnotation {
  accessor veryLongPropertyName: Map<string, SomeReallyLongTypeName> = new Map<string, SomeReallyLongTypeName>();
}

export class longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass {}

export interface longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass {}

let longRunningProvider = new (class
  implements languages.SignatureHelpProvider
{});

let longRunningProvider2 = new (class
  extends languages.SignatureHelpProvider
{});

let longRunningProvider3 = new (class
  extends languages.SignatureHelpProvider<Hello>
{});

let longRunningProvider4 = new (class
  implements languages.SignatureHelpProvider<Hello>
{});

letlonglongRunningProvider = class
  implements languages.SignatureHelpProvider
{};

letlonglongRunningProvider2 = class
  extends languages.SignatureHelpProvider
{};

letlonglongRunningProvider3 = class
  extends languages.SignatureHelpProvider<Hello>
{};

letlonglongRunningProvider4 = class
  implements languages.SignatureHelpProvider<Hello>
{};
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// Accessor with type annotation
class Typed {
  accessor foo: string = "bar";
  accessor veryLongPropertyName: SomeReallyLongTypeName =
    someReallyLongFunctionName(argumentOne, argumentTwo);
}

// Accessor with modifiers
class Modifiers {
  public accessor foo: string = "bar";
  private accessor bar: number = 42;
  protected accessor baz: boolean = true;
  static accessor qux: string = "static";
  override accessor overridden: string = "overridden";
}

// Abstract accessor
abstract class AbstractAccessor {
  abstract accessor foo: string;
}

// Accessor with definite assignment
class Definite {
  accessor foo!: string;
}

// Accessor with long type annotation and value
class LongAnnotation {
  accessor veryLongPropertyName: Map<string, SomeReallyLongTypeName> = new Map<
    string,
    SomeReallyLongTypeName
  >();
}

export class longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass {}

export interface longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass {}

let longRunningProvider = new (class
  implements languages.SignatureHelpProvider {})();

let longRunningProvider2 = new (class
  extends languages.SignatureHelpProvider {})();

let longRunningProvider3 =
  new (class extends languages.SignatureHelpProvider<Hello> {})();

let longRunningProvider4 =
  new (class implements languages.SignatureHelpProvider<Hello> {})();

letlonglongRunningProvider = class
  implements languages.SignatureHelpProvider {};

letlonglongRunningProvider2 = class extends languages.SignatureHelpProvider {};

letlonglongRunningProvider3 = class extends (
  languages.SignatureHelpProvider<Hello>
) {};

letlonglongRunningProvider4 = class implements languages.SignatureHelpProvider<Hello> {};
"#,
            ),
            (
                100,
                r#"// Accessor with type annotation
class Typed {
  accessor foo: string = "bar";
  accessor veryLongPropertyName: SomeReallyLongTypeName = someReallyLongFunctionName(
    argumentOne,
    argumentTwo,
  );
}

// Accessor with modifiers
class Modifiers {
  public accessor foo: string = "bar";
  private accessor bar: number = 42;
  protected accessor baz: boolean = true;
  static accessor qux: string = "static";
  override accessor overridden: string = "overridden";
}

// Abstract accessor
abstract class AbstractAccessor {
  abstract accessor foo: string;
}

// Accessor with definite assignment
class Definite {
  accessor foo!: string;
}

// Accessor with long type annotation and value
class LongAnnotation {
  accessor veryLongPropertyName: Map<string, SomeReallyLongTypeName> = new Map<
    string,
    SomeReallyLongTypeName
  >();
}

export class longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass {}

export interface longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass {}

let longRunningProvider = new (class implements languages.SignatureHelpProvider {})();

let longRunningProvider2 = new (class extends languages.SignatureHelpProvider {})();

let longRunningProvider3 = new (class extends languages.SignatureHelpProvider<Hello> {})();

let longRunningProvider4 = new (class implements languages.SignatureHelpProvider<Hello> {})();

letlonglongRunningProvider = class implements languages.SignatureHelpProvider {};

letlonglongRunningProvider2 = class extends languages.SignatureHelpProvider {};

letlonglongRunningProvider3 = class extends languages.SignatureHelpProvider<Hello> {};

letlonglongRunningProvider4 = class implements languages.SignatureHelpProvider<Hello> {};
"#,
            ),
        ],
    );
}

/// Member decorators should stay on their own line in class bodies.
#[test]
fn test_format_class_decorator_layout() {
    assert_format_program!(
        r#"class A {
  // comment shouldn't break the decorators grouping
  @memoize onContextMenu() { }
}
"#,
        r#"class A {
    // comment shouldn't break the decorators grouping
    @memoize
    onContextMenu() {}
}
"#,
        FileType::TypeScript,
    );
}

/// Decorator chain comments should stay interleaved without extra blank lines.
#[test]
fn test_format_member_decorator_comment_layout() {
    assert_format_program_reference_widths(
        r#"class Box {
  // comment before entity
  @entity
  // comment after entity
  // comment before foo
  @foo(1, 2, 3)
  // comment after foo
  method() {}
}
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"class Box {
  // comment before entity
  @entity
  // comment after entity
  // comment before foo
  @foo(1, 2, 3)
  // comment after foo
  method() {}
}
"#,
            ),
            (
                100,
                r#"class Box {
  // comment before entity
  @entity
  // comment after entity
  // comment before foo
  @foo(1, 2, 3)
  // comment after foo
  method() {}
}
"#,
            ),
        ],
    );
}
