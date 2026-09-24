use crate::{
    DestackFormatOptions, assert_format, assert_format_program,
    assert_format_program_reference_widths, parse_first_expression,
};
use destack_source::FileType;

/// An empty enum formats without a space between its braces.
#[test]
fn test_format_enum_empty() {
    assert_format!(
        "enum { }",
        "enum {}",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Malformed generic argument slots should preserve their authored source.
#[test]
fn test_format_recovered_generic_argument() {
    assert_format!(
        "type Value=Container<>",
        "type Value = Container<>;",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Malformed type members should preserve their authored source.
#[test]
fn test_format_recovered_type_member() {
    assert_format!(
        "interface Value {\n+\ny: int32\n}",
        "interface Value {\n    +;\n    y: int32;\n}",
        parse_first_expression,
        DestackFormatOptions::default(),
    );
}

/// Shared and export modifiers format ahead of nominal declarations.
#[test]
fn test_format_shared_nominal_declarations() {
    assert_format_program!(
        r#"class Promise<T>{}
export default class Deferred<T>{}
shared struct Channel<T>{}
newtype interface Awaitable<T> {}
shared enum Result { Ok; Error }
enum Mode { Read; Write }
newtype TaskId = uint64
"#,
        r#"class Promise<T> {}
export default class Deferred<T> {}
shared struct Channel<T> {}
newtype interface Awaitable<T> {}
shared enum Result {
    Ok,
    Error,
}
enum Mode {
    Read,
    Write,
}
newtype TaskId = uint64;
"#,
        FileType::Destack,
    );
}

/// An enum with bare members formats one member per line.
#[test]
fn test_format_enum_with_simple_fields() {
    assert_format!(
        "enum { A, B }",
        r#"enum {
	A,
	B,
}"#,
        parse_first_expression,
        DestackFormatOptions::default_tab()
    );
}

/// Enum annotations format on their own lines above each member.
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

/// An enum with generic parameters and defaults keeps its authored form.
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
        parse_first_expression,
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
    /// 8-bit signed integer
    i8 = 1,

    /// 8-bit unsigned integer
    u8 = 2,
}
"#,
        FileType::DestackDeclaration,
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
    /// Property to get and set the pipelining factor.
    pipelining: number;
    /// `true` after `client.close()` has been called.
    closed: boolean;
}
"#,
        FileType::DestackDeclaration,
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
    /// Get hash field values with expiration options
    //prettier-ignore
    hgetex(key: KeyLike, fieldsKeyword: "FIELDS", numfields: number, ...fields: KeyLike[]): Promise<Array<string | null>>;
}
"#,
        FileType::DestackDeclaration,
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

// Accessor without initializer
class Declared {
  accessor foo: string;
}

// Accessor with long type annotation and value
class LongAnnotation {
  accessor veryLongPropertyName: Map<string, SomeReallyLongTypeName> = new Map<string, SomeReallyLongTypeName>();
}

export class longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass {}

export interface longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass {}

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
        FileType::Destack,
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

// Accessor without initializer
class Declared {
  accessor foo: string;
}

// Accessor with long type annotation and value
class LongAnnotation {
  accessor veryLongPropertyName: Map<string, SomeReallyLongTypeName> = new Map<
    string,
    SomeReallyLongTypeName
  >();
}

export class longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass
{}

export interface longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass
{}

letlonglongRunningProvider = class
  implements languages.SignatureHelpProvider
{};

letlonglongRunningProvider2 = class extends languages.SignatureHelpProvider {};

letlonglongRunningProvider3 = class extends (
  languages.SignatureHelpProvider<Hello>
)
{};

letlonglongRunningProvider4 = class implements languages.SignatureHelpProvider<Hello>
{};
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

// Accessor without initializer
class Declared {
  accessor foo: string;
}

// Accessor with long type annotation and value
class LongAnnotation {
  accessor veryLongPropertyName: Map<string, SomeReallyLongTypeName> = new Map<
    string,
    SomeReallyLongTypeName
  >();
}

export class longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass
{}

export interface longlonglonglonglonglonglonglonglonglonglongclassname
  extends someobjectsomepropertysomeotherproperty.SomeClass
{}

letlonglongRunningProvider = class implements languages.SignatureHelpProvider {};

letlonglongRunningProvider2 = class extends languages.SignatureHelpProvider {};

letlonglongRunningProvider3 = class extends languages.SignatureHelpProvider<Hello> {};

letlonglongRunningProvider4 = class implements languages.SignatureHelpProvider<Hello> {};
"#,
            ),
        ],
    );
}

/// Extension heritage lists should indent every broken item under `implements`.
#[test]
fn test_format_extension_implements_list_layout() {
    assert_format_program_reference_widths(
        r#"extension<T> of Deque<T> implements
    Index<number>,
    IndexSet<number, T>,
    Iterable<T>,
    Iterable<&readonly T>,
    Extend<T, "mutable">
{
    index(index: number): T;
}
"#,
        FileType::Destack,
        &[
            (
                80,
                r#"extension<T> of Deque<T>
  implements
    Index<number>,
    IndexSet<number, T>,
    Iterable<T>,
    Iterable<&readonly T>,
    Extend<T, "mutable">
{
  index(index: number): T;
}
"#,
            ),
            (
                160,
                r#"extension<T> of Deque<T> implements Index<number>, IndexSet<number, T>, Iterable<T>, Iterable<&readonly T>, Extend<T, "mutable"> {
  index(index: number): T;
}
"#,
            ),
        ],
    );
}

/// Generic heritage items should break inside the indented heritage list.
#[test]
fn test_format_extension_implements_generic_item_layout() {
    assert_format_program_reference_widths(
        r#"extension<R> of X implements IndexSet<VeryLongCoordinateName<R>, VeryLongSliceName<R>> where R: Copy {
    indexSet(coordinate: VeryLongCoordinateName<R>, value: VeryLongSliceName<R>): void;
}
"#,
        FileType::Destack,
        &[
            (
                80,
                r#"extension<R> of X
  implements IndexSet<VeryLongCoordinateName<R>, VeryLongSliceName<R>>
  where R: Copy
{
  indexSet(
    coordinate: VeryLongCoordinateName<R>,
    value: VeryLongSliceName<R>,
  ): void;
}
"#,
            ),
            (
                120,
                r#"extension<R> of X implements IndexSet<VeryLongCoordinateName<R>, VeryLongSliceName<R>> where R: Copy {
  indexSet(coordinate: VeryLongCoordinateName<R>, value: VeryLongSliceName<R>): void;
}
"#,
            ),
        ],
    );
}

/// Long extension targets break after `of`, keeping their generic arguments on one line.
#[test]
fn test_format_extension_target_type_layout() {
    assert_format_program_reference_widths(
        r#"extension<T, const Rank: int, F: TensorFormat, const ...Axes: ShardingAxis> of Tensor<T, Rank, F, Sharding<...Axes>> {
}
"#,
        FileType::Destack,
        &[
            (
                100,
                r#"extension<T, const Rank: int, F: TensorFormat, const ...Axes: ShardingAxis> of
  Tensor<T, Rank, F, Sharding<...Axes>>
{}
"#,
            ),
            (
                140,
                r#"extension<T, const Rank: int, F: TensorFormat, const ...Axes: ShardingAxis> of Tensor<T, Rank, F, Sharding<...Axes>> {}
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
        FileType::Destack,
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
        FileType::Destack,
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

/// Named borrow lifetimes should keep their written tick form.
#[test]
fn test_format_borrow_lifetime_roundtrip() {
    assert_format!(
        "type View = &'a readonly Buffer;",
        "type View = &'a readonly Buffer;",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// The static lifetime literal should keep its tick form.
#[test]
fn test_format_static_lifetime_roundtrip() {
    assert_format!(
        "type View = &'static Buffer;",
        "type View = &'static Buffer;",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Bare tick generic parameters should print without const.
#[test]
fn test_format_bare_lifetime_parameter_roundtrip() {
    assert_format!(
        "function first<'a>(a: &'a Node, b: &Node): &'a Node {\n    return a;\n}",
        "function first<'a>(a: &'a Node, b: &Node): &'a Node {\n    return a;\n}",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Explicit const lifetime bounds should normalize to the bare tick name.
#[test]
fn test_format_const_lifetime_parameter_normalizes_bare() {
    assert_format!(
        "declare function only<const 'a: Lifetime>(value: Borrowed<Node, 'a>): Borrowed<Node, 'a>;",
        "declare function only<'a>(value: Borrowed<Node, 'a>): Borrowed<Node, 'a>;",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Lifetime unions should print as ordinary type algebra.
#[test]
fn test_format_lifetime_union_roundtrip() {
    assert_format!(
        "type Joined = Borrowed<Node, 'a | 'b>;",
        "type Joined = Borrowed<Node, 'a | 'b>;",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Region meets should print as ordinary intersection algebra.
#[test]
fn test_format_region_meet_roundtrip() {
    assert_format!(
        "type Leaked = Borrowed<Node, 'static & S>;",
        "type Leaked = Borrowed<Node, 'static & S>;",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Confined existentials should print the region operand inside the intersection.
#[test]
fn test_format_dynamic_region_roundtrip() {
    assert_format!(
        "type Confined = Dynamic<Printable & 'a>;",
        "type Confined = Dynamic<Printable & 'a>;",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Outlives clauses should print tick operands on both sides.
#[test]
fn test_format_where_outlives_roundtrip() {
    assert_format!(
        "declare function only<'a, 'b>(value: &'a Node): &'a Node where 'a: 'b;",
        "declare function only<'a, 'b>(value: &'a Node): &'a Node where 'a: 'b;",
        parse_first_expression,
        DestackFormatOptions::default()
    );
}
