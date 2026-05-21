use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_evaluates_conditional_type_instances() {
    let session = TestSession::single(
        r#"
type Select<T> = T extends string ? string : int32;

declare const text: Select<string>;
declare const number: Select<boolean>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Select<T> = T extends string ? string : int32;
/// @generic.slot symbol=Select.T index=0 kind=type
/// @type.symbol symbol=Select type=T extends string ? string : int32

declare const text: Select<string>;
/// @resolution.name source=Select target=Select
/// @instance.application source="Select<string>" id=Select<string>
/// @type.symbol symbol=text type=string

declare const number: Select<boolean>;
/// @resolution.name source=Select target=Select
/// @instance.application source="Select<boolean>" id=Select<boolean>
/// @type.symbol symbol=number type=int32

/// @instance.entry id=Select<string> symbol=Select arguments=[string]
/// @instance.entry id=Select<boolean> symbol=Select arguments=[boolean]
"#,
    );
}

#[test]
fn test_check_evaluates_keyof_and_indexed_access_types() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Keys = keyof User;
type Name = User["name"];
type Value = User["name" | "age"];

declare const key: Keys;
declare const name: Name;
declare const value: Value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type User = { name: string; age: int32 };
/// @type.symbol symbol=User type={ name: string; age: int32 }

type Keys = keyof User;
/// @resolution.name source=User target=User
/// @type.symbol symbol=Keys type="name" | "age"

type Name = User["name"];
/// @resolution.name source=User target=User
/// @type.symbol symbol=Name type=string

type Value = User["name" | "age"];
/// @resolution.name source=User target=User
/// @type.symbol symbol=Value type=string | int32

declare const key: Keys;
/// @resolution.name source=Keys target=Keys
/// @type.symbol symbol=key type="name" | "age"

declare const name: Name;
/// @resolution.name source=Name target=Name
/// @type.symbol symbol=name type=string

declare const value: Value;
/// @resolution.name source=Value target=Value
/// @type.symbol symbol=value type=string | int32
"#,
    );
}

#[test]
fn test_check_reports_missing_indexed_access_keys() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Missing = User["missing"];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
type User = { name: string; age: int32 };
/// @type.symbol symbol=User type={ name: string; age: int32 }

type Missing = User["missing"];
/// @resolution.name source=User target=User

"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'missing'"
/// @diagnostic.label line=3 column=21 source="type Missing = User[\"missing\"];"
"#,
    );
}

#[test]
fn test_check_evaluates_mapped_type_instances() {
    let session = TestSession::single(
        r#"
type Flags<T> = { [K in keyof T]: boolean };
type Actual = Flags<{ name: string; age: int32 }>;

declare const value: Actual;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Flags<T> = { [K in keyof T]: boolean };
/// @generic.slot symbol=Flags.T index=0 kind=type
/// @type.symbol symbol=Flags type={ [K in keyof T]: boolean }

type Actual = Flags<{ name: string; age: int32 }>;
/// @resolution.name source=Flags target=Flags
/// @instance.application source="Flags<{ name: string; age: int32 }>" id="Flags<{ name: string; age: int32 }>"
/// @type.symbol symbol=Actual type={ name: boolean; age: boolean }

declare const value: Actual;
/// @resolution.name source=Actual target=Actual
/// @type.symbol symbol=value type={ name: boolean; age: boolean }

/// @instance.entry id="Flags<{ name: string; age: int32 }>" symbol=Flags arguments=[{ name: string; age: int32 }]
"#,
    );
}

#[test]
fn test_check_evaluates_template_literal_infer_types() {
    let session = TestSession::single(
        r#"
type Segment<T> = T extends `/${infer Name}` ? Name : never;
type Name = Segment<"/api">;

declare const name: Name;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Segment<T> = T extends `/${infer Name}` ? Name : never;
/// @generic.slot symbol=Segment.T index=0 kind=type
/// @type.symbol symbol=Segment type=T extends `/${infer Name}` ? Name : never

type Name = Segment<"/api">;
/// @resolution.name source=Segment target=Segment
/// @instance.application source="Segment<\"/api\">" id="Segment<\"/api\">"
/// @type.symbol symbol=Name type="api"

declare const name: Name;
/// @resolution.name source=Name target=Name
/// @type.symbol symbol=name type="api"

/// @instance.entry id="Segment<\"/api\">" symbol=Segment arguments=["/api"]
"#,
    );
}

#[test]
fn test_check_merges_intersection_object_shapes() {
    let session = TestSession::single(
        r#"
type Named = { name: string };
type Aged = { age: int32 };
type Person = Named & Aged;

declare const person: Person;
const name = person.name;
const age = person.age;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Named = { name: string };
/// @type.symbol symbol=Named type={ name: string }

type Aged = { age: int32 };
/// @type.symbol symbol=Aged type={ age: int32 }

type Person = Named & Aged;
/// @resolution.name source=Named target=Named
/// @resolution.name source=Aged target=Aged
/// @type.symbol symbol=Person type={ name: string; age: int32 }

declare const person: Person;
/// @resolution.name source=Person target=Person
/// @type.symbol symbol=person type={ name: string; age: int32 }

const name = person.name;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string; age: int32 } kind=symbol target=person.name
/// @type.symbol symbol=name type=string

const age = person.age;
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver={ name: string; age: int32 } kind=symbol target=person.age
/// @type.symbol symbol=age type=int32
"#,
    );
}

#[test]
fn test_check_preserves_interval_types_without_exploding_unions() {
    let session = TestSession::single(
        r#"
type Count = 0..5;

declare const count: Count;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Count = 0..5;
/// @type.symbol symbol=Count type=0..5

declare const count: Count;
/// @resolution.name source=Count target=Count
/// @type.symbol symbol=count type=0..5
"#,
    );
}

#[test]
fn test_check_evaluates_builtin_string_type_functions() {
    let session = TestSession::single(
        r#"
type Upper = Uppercase<"hello">;
type Lower = Lowercase<"HELLO">;
type Title = Capitalize<"hello">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Upper = Uppercase<"hello">;
/// @type.symbol symbol=Upper type="HELLO"

type Lower = Lowercase<"HELLO">;
/// @type.symbol symbol=Lower type="hello"

type Title = Capitalize<"hello">;
/// @type.symbol symbol=Title type="Hello"
"#,
    );
}
