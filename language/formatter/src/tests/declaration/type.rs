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
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_enum_with_simple_fields() {
    assert_format!(
        "enum { A, B }",
        "enum {\n\tA,\n\tB,\n}",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_enum_with_annotations() {
    assert_format_program!(
        r#"@description("The status of a task.") enum Status { @default Todo; Done }"#,
        r#"@description("The status of a task.") enum Status {
    @default Todo,
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
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
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
