use crate::tests::{DirRows, TestSession};

/// An indexed access projects the type of an object property.
#[test]
fn test_indexed_access_projects_object_property_type() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Name = User["name"];

declare const name: Name;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Name = User["name"];

declare const name: Name;

=== dir ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }
/// @type.symbol symbol=User.name source="name: string" type=string
/// @type.symbol symbol=User.age source="age: int32" type=int32

type Name = User["name"];
/// @type.symbol symbol=Name source="type Name = User[\"name\"]" type=string
/// @definition.type symbol=Name source="type Name = User[\"name\"]" value=User["name"]
/// @resolution.name source=User target=User

declare const name: Name;
/// @type.symbol symbol=name source=name type=Name
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=Name target=Name
"#,
    );
}

/// An indexed access projects an object property named by a usize key.
#[test]
fn test_indexed_access_projects_usize_object_key() {
    let session = TestSession::single(
        r#"
type Pair = { 0: string; 1: int32 };
type Right = Pair[1];

declare const value: Right;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair = { 0: string; 1: int32 };
type Right = Pair[1];

declare const value: Right;

=== dir ===
type Pair = { 0: string; 1: int32 };
/// @type.symbol symbol=Pair source="type Pair = { 0: string; 1: int32 }" type={ 0: string; 1: int32 }
/// @definition.type symbol=Pair source="type Pair = { 0: string; 1: int32 }" value={ 0: string; 1: int32 }
/// @type.symbol symbol=Pair.symbol2 source="0: string" type=string
/// @type.symbol symbol=Pair.symbol4 source="1: int32" type=int32

type Right = Pair[1];
/// @type.symbol symbol=Right source="type Right = Pair[1]" type=int32
/// @definition.type symbol=Right source="type Right = Pair[1]" value=Pair[1]
/// @resolution.name source=Pair target=Pair

declare const value: Right;
/// @type.symbol symbol=value source=value type=Right
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Right target=Right
"#,
    );
}

/// An indexed access with a missing usize key reports a diagnostic.
#[test]
fn test_indexed_access_rejects_missing_usize_object_key() {
    let session = TestSession::single(
        r#"
type ObjectLike = { label: string };
type Missing = ObjectLike[5];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type ObjectLike = { label: string };
type Missing = ObjectLike[5];

=== dir ===
type ObjectLike = { label: string };
/// @type.symbol symbol=ObjectLike source="type ObjectLike = { label: string }" type={ label: string }
/// @definition.type symbol=ObjectLike source="type ObjectLike = { label: string }" value={ label: string }
/// @type.symbol symbol=ObjectLike.label source="label: string" type=string

type Missing = ObjectLike[5];
/// @type.symbol symbol=Missing source="type Missing = ObjectLike[5]" type=<error>
/// @definition.type symbol=Missing source="type Missing = ObjectLike[5]" value=ObjectLike[5]
/// @resolution.name source=ObjectLike target=ObjectLike
"#,
        r#"
/// @diagnostic.error id=invalid-index-key message="type '{ label: string }' cannot be indexed by type '5'"
/// @diagnostic.label line=3 column=16 span="ObjectLike[5]" line_source="type Missing = ObjectLike[5];"
"#,
    );
}

/// An indexed access on a scalar receiver reports a diagnostic.
#[test]
fn test_indexed_access_rejects_non_indexable_receiver() {
    let session = TestSession::single(
        r#"
type Missing = int32["name"];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Missing = int32["name"];

=== dir ===
type Missing = int32["name"];
/// @type.symbol symbol=Missing source="type Missing = int32[\"name\"]" type=<error>
/// @definition.type symbol=Missing source="type Missing = int32[\"name\"]" value=int32["name"]
"#,
        r#"
/// @diagnostic.error id=invalid-index-receiver message="type 'int32' cannot be indexed"
/// @diagnostic.label line=2 column=16 span="int32[\"name\"]" line_source="type Missing = int32[\"name\"];"
"#,
    );
}

/// An indexed access over a union of keys projects the union of their values.
#[test]
fn test_indexed_access_projects_key_union() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Value = User["name" | "age"];

declare const value: Value;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Value = User["name" | "age"];

declare const value: Value;

=== dir ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }
/// @type.symbol symbol=User.name source="name: string" type=string
/// @type.symbol symbol=User.age source="age: int32" type=int32

type Value = User["name" | "age"];
/// @type.symbol symbol=Value source="type Value = User[\"name\" | \"age\"]" type=string | int32
/// @definition.type symbol=Value source="type Value = User[\"name\" | \"age\"]" value=User["name" | "age"]
/// @resolution.name source=User target=User

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
    );
}

/// A value outside the projected key union reports a diagnostic.
#[test]
fn test_indexed_access_key_union_rejects_unselected_value() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Value = User["name" | "age"];

const bad: Value = true;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Value = User["name" | "age"];

const bad: Value = true;

=== dir ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }
/// @type.symbol symbol=User.name source="name: string" type=string
/// @type.symbol symbol=User.age source="age: int32" type=int32

type Value = User["name" | "age"];
/// @type.symbol symbol=Value source="type Value = User[\"name\" | \"age\"]" type=string | int32
/// @definition.type symbol=Value source="type Value = User[\"name\" | \"age\"]" value=User["name" | "age"]
/// @resolution.name source=User target=User

const bad: Value = true;
/// @type.symbol symbol=bad source=bad type=Value
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'true' is not assignable to type 'Value'"
/// @diagnostic.label line=5 column=20 span="true" line_source="const bad: Value = true;"
/// @diagnostic.related line=5 column=12 span="Value" line_source="const bad: Value = true;" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to 'string | int32'"
"#,
    );
}

/// An indexed access distributes over each arm of a union receiver.
#[test]
fn test_indexed_access_distributes_over_union_values() {
    let session = TestSession::single(
        r#"
type Left = { kind: "left"; value: int32 };
type Right = { kind: "right"; value: string };
type Value = (Left | Right)["value"];

const number: Value = 1;
const text: Value = "hello";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Left = { kind: "left"; value: int32 };
type Right = { kind: "right"; value: string };
type Value = (Left | Right)["value"];

const number: Value = 1 as Value;
const text: Value = "hello" as Value;

=== dir ===
type Left = { kind: "left"; value: int32 };
/// @type.symbol symbol=Left source="type Left = { kind: \"left\"; value: int32 }" type={ kind: "left"; value: int32 }
/// @definition.type symbol=Left source="type Left = { kind: \"left\"; value: int32 }" value={ kind: "left"; value: int32 }
/// @type.symbol symbol=Left.kind source="kind: \"left\"" type="left"
/// @type.symbol symbol=Left.value source="value: int32" type=int32

type Right = { kind: "right"; value: string };
/// @type.symbol symbol=Right source="type Right = { kind: \"right\"; value: string }" type={ kind: "right"; value: string }
/// @definition.type symbol=Right source="type Right = { kind: \"right\"; value: string }" value={ kind: "right"; value: string }
/// @type.symbol symbol=Right.kind source="kind: \"right\"" type="right"
/// @type.symbol symbol=Right.value source="value: string" type=string

type Value = (Left | Right)["value"];
/// @type.symbol symbol=Value source="type Value = (Left | Right)[\"value\"]" type=int32 | string
/// @definition.type symbol=Value source="type Value = (Left | Right)[\"value\"]" value=Left | Right["value"]
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

const number: Value = 1;
/// @type.symbol symbol=number source=number type=Value
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=Value target=Value

const text: Value = "hello";
/// @type.symbol symbol=text source=text type=Value
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=Value target=Value
"#,
    );
}

/// An indexed access over an optional union member includes undefined.
#[test]
fn test_indexed_access_optional_union_member_includes_undefined() {
    let session = TestSession::single(
        r#"
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
type Value = Input["value"];

const missing: Value = undefined;
const number: Value = 1;
const text: Value = "hello";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
type Value = Input["value"];

const missing: Value = undefined as Value;
const number: Value = 1 as Value;
const text: Value = "hello" as Value;

=== dir ===
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
/// @type.symbol symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" type={ kind: "a"; value?: float64 } | { kind: "b"; value: string }
/// @definition.type symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" value={ kind: "a"; value?: float64 } | { kind: "b"; value: string }
/// @type.symbol symbol=Input.kind#1 source="kind: \"a\"" type="a"
/// @type.symbol symbol=Input.value#1 source="value?: number" type=float64
/// @type.symbol symbol=Input.kind#2 source="kind: \"b\"" type="b"
/// @type.symbol symbol=Input.value#2 source="value: string" type=string

type Value = Input["value"];
/// @type.symbol symbol=Value source="type Value = Input[\"value\"]" type=float64 | undefined | string
/// @definition.type symbol=Value source="type Value = Input[\"value\"]" value=Input["value"]
/// @resolution.name source=Input target=Input

const missing: Value = undefined;
/// @type.symbol symbol=missing source=missing type=Value
/// @resolution.pattern source=missing kind=binding target=missing
/// @resolution.name source=Value target=Value

const number: Value = 1;
/// @type.symbol symbol=number source=number type=Value
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=Value target=Value

const text: Value = "hello";
/// @type.symbol symbol=text source=text type=Value
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=Value target=Value
"#,
    );
}

/// An unrelated value reports a diagnostic at an optional union member projection.
#[test]
fn test_indexed_access_optional_union_member_rejects_unrelated_value() {
    let session = TestSession::single(
        r#"
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
type Value = Input["value"];

const bad: Value = true;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
type Value = Input["value"];

const bad: Value = true;

=== dir ===
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
/// @type.symbol symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" type={ kind: "a"; value?: float64 } | { kind: "b"; value: string }
/// @definition.type symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" value={ kind: "a"; value?: float64 } | { kind: "b"; value: string }
/// @type.symbol symbol=Input.kind#1 source="kind: \"a\"" type="a"
/// @type.symbol symbol=Input.value#1 source="value?: number" type=float64
/// @type.symbol symbol=Input.kind#2 source="kind: \"b\"" type="b"
/// @type.symbol symbol=Input.value#2 source="value: string" type=string

type Value = Input["value"];
/// @type.symbol symbol=Value source="type Value = Input[\"value\"]" type=float64 | undefined | string
/// @definition.type symbol=Value source="type Value = Input[\"value\"]" value=Input["value"]
/// @resolution.name source=Input target=Input

const bad: Value = true;
/// @type.symbol symbol=bad source=bad type=Value
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'true' is not assignable to type 'Value'"
/// @diagnostic.label line=5 column=20 span="true" line_source="const bad: Value = true;"
/// @diagnostic.related line=5 column=12 span="Value" line_source="const bad: Value = true;" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to 'float64 | undefined | string'"
"#,
    );
}

/// An indexed access with a missing object key reports a diagnostic.
#[test]
fn test_indexed_access_rejects_missing_object_key() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Missing = User["missing"];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Missing = User["missing"];

=== dir ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }
/// @type.symbol symbol=User.name source="name: string" type=string
/// @type.symbol symbol=User.age source="age: int32" type=int32

type Missing = User["missing"];
/// @type.symbol symbol=Missing source="type Missing = User[\"missing\"]" type=<error>
/// @definition.type symbol=Missing source="type Missing = User[\"missing\"]" value=User["missing"]
/// @resolution.name source=User target=User
"#,
        r#"
/// @diagnostic.error id=invalid-index-key message="type '{ name: string; age: int32 }' cannot be indexed by type '\"missing\"'"
/// @diagnostic.label line=3 column=16 span="User[\"missing\"]" line_source="type Missing = User[\"missing\"];"
"#,
    );
}

/// An indexed access projects the type at a tuple position.
#[test]
fn test_indexed_access_projects_tuple_position() {
    let session = TestSession::single(
        r#"
type Pair = (string, int32);
type First = Pair[0];

declare const first: First;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair = (string, int32);
type First = Pair[0];

declare const first: First;

=== dir ===
type Pair = (string, int32);
/// @type.symbol symbol=Pair source="type Pair = (string, int32)" type=(string, int32)
/// @definition.type symbol=Pair source="type Pair = (string, int32)" value=(string, int32)

type First = Pair[0];
/// @type.symbol symbol=First source="type First = Pair[0]" type=string
/// @definition.type symbol=First source="type First = Pair[0]" value=Pair[0]
/// @resolution.name source=Pair target=Pair

declare const first: First;
/// @type.symbol symbol=first source=first type=First
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=First target=First
"#,
    );
}

/// An indexed access with a usize key projects the element of an array.
#[test]
fn test_indexed_access_projects_dynamic_array_element() {
    let session = TestSession::single(
        r#"
type Element<T: string[]> = T[usize];
type Value = Element<string[]>;

declare const value: Value;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Element<T: string[]> = T[usize];
type Value = Element<string[]>;

declare const value: Value;

=== dir ===
type Element<T: string[]> = T[usize];
/// @generic.template symbol=Element parameters=(T: string[])
/// @type.symbol symbol=Element source="type Element<T: string[]> = T[usize]" type=T[usize]
/// @definition.type symbol=Element source="type Element<T: string[]> = T[usize]" template=(T: string[]) value=T[usize]
/// @type.symbol symbol=Element.T source="T: string[]" type=T
/// @resolution.name source=T target=Element.T

type Value = Element<string[]>;
/// @type.symbol symbol=Value source="type Value = Element<string[]>" type=string
/// @generic.instance id=Array<string> template=Array arguments=(string)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<string>> template=sliceAssumeInit arguments=(MaybeUninit<string>)
/// @generic.instance id=sliceUninit<MaybeUninit<string>> template=sliceUninit arguments=(MaybeUninit<string>)
/// @definition.type symbol=Value source="type Value = Element<string[]>" value=Element<string[]>
/// @resolution.name source=Element target=Element

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
    );
}

/// A generic indexed access projects through a key bounded by keyof.
#[test]
fn test_generic_indexed_access_projects_key_constraint() {
    let session = TestSession::single(
        r#"
type ValueAt<T, K: keyof T> = T[K];
type User = { name: string; age: int32 };
type Name = ValueAt<User, "name">;

declare const name: Name;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type ValueAt<T, K: keyof T> = T[K];
type User = { name: string; age: int32 };
type Name = ValueAt<User, "name">;

declare const name: Name;

=== dir ===
type ValueAt<T, K: keyof T> = T[K];
/// @generic.template symbol=ValueAt parameters=(T, K: keyof T)
/// @type.symbol symbol=ValueAt source="type ValueAt<T, K: keyof T> = T[K]" type=T[K]
/// @definition.type symbol=ValueAt source="type ValueAt<T, K: keyof T> = T[K]" template=(T, K: keyof T) value=T[K]
/// @type.symbol symbol=ValueAt.T source=T type=T
/// @type.symbol symbol=ValueAt.K source="K: keyof T" type=K
/// @resolution.name source=T target=ValueAt.T
/// @resolution.name source=T target=ValueAt.T
/// @resolution.name source=K target=ValueAt.K

type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }
/// @type.symbol symbol=User.name source="name: string" type=string
/// @type.symbol symbol=User.age source="age: int32" type=int32

type Name = ValueAt<User, "name">;
/// @type.symbol symbol=Name source="type Name = ValueAt<User, \"name\">" type=string
/// @definition.type symbol=Name source="type Name = ValueAt<User, \"name\">" value=ValueAt<User, "name">
/// @resolution.name source=ValueAt target=ValueAt
/// @resolution.name source=User target=User

declare const name: Name;
/// @type.symbol symbol=name source=name type=Name
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=Name target=Name
"#,
    );
}

/// A generic indexed access with an unbounded key reports a diagnostic.
#[test]
fn test_generic_indexed_access_rejects_unconstrained_key() {
    let session = TestSession::single(
        r#"
type ValueAt<T, K> = T[K];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type ValueAt<T, K> = T[K];

=== dir ===
type ValueAt<T, K> = T[K];
/// @generic.template symbol=ValueAt parameters=(T, K)
/// @type.symbol symbol=ValueAt source="type ValueAt<T, K> = T[K]" type=T[K]
/// @definition.type symbol=ValueAt source="type ValueAt<T, K> = T[K]" template=(T, K) value=T[K]
/// @type.symbol symbol=ValueAt.T source=T type=T
/// @type.symbol symbol=ValueAt.K source=K type=K
/// @resolution.name source=T target=ValueAt.T
/// @resolution.name source=K target=ValueAt.K
"#,
        r#"
/// @diagnostic.error id=invalid-index-key message="type 'T' cannot be indexed by type 'K'"
/// @diagnostic.label line=2 column=22 span="T[K]" line_source="type ValueAt<T, K> = T[K];"
"#,
    );
}

/// A keyof-bounded parameter indexes an object and keeps each value type.
#[test]
fn test_generic_keyof_parameter_indexes_object_value() {
    let session = TestSession::single(
        r#"
type User = {
    readonly name: string;
    readonly age: int32;
};

function get<K: keyof User>(user: User, key: K): User[K] {
    return user[key];
}

declare const user: User;
const name = get(user, "name");
const age = get(user, "age");

name satisfies string;
age satisfies int32;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type User = {
    readonly name: string;
    readonly age: int32;
};

function get<K: keyof User>(user: User, key: K): User[K] {
    return user[key];
}

declare const user: User;
const name: string = get<"name">(user, "name");
const age: int32 = get<"age">(user, "age");

name satisfies string;
age satisfies int32;

=== dir ===
type User = {
/// @type.symbol symbol=User type={ readonly name: string; readonly age: int32 }
/// @definition.type symbol=User value={ readonly name: string; readonly age: int32 }

    readonly name: string;
    /// @type.symbol symbol=User.name source="readonly name: string" type=string

    readonly age: int32;
    /// @type.symbol symbol=User.age source="readonly age: int32" type=int32

};

function get<K: keyof User>(user: User, key: K): User[K] {
/// @generic.template symbol=get parameters=(K: keyof User)
/// @type.symbol symbol=get type=<K: keyof User>(User, K) => User[K]
/// @type.symbol symbol=get.K source="K: keyof User" type=K
/// @resolution.name source=User target=User
/// @type.symbol symbol=get.user source="user: User" type=User
/// @resolution.name source=User target=User
/// @type.symbol symbol=get.key source="key: K" type=K
/// @resolution.name source=K target=get.K
/// @resolution.name source=User target=User
/// @resolution.name source=K target=get.K

    return user[key];
    /// @type.node source=user type=User
    /// @type.node source=user[key] type={ readonly name: string; readonly age: int32 }[K]
    /// @resolution.name source=user target=get.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=get.user
    /// @resolution.place source=user[key] placement="local" lifetime="managed" access="mutable"
    /// @resolution.subscript source=user[key] type={ readonly name: string; readonly age: int32 }[K] kind=member target="receiver=User, target=index(keyof { readonly name: string; readonly age: int32 }), type={ readonly name: string; readonly age: int32 }[K]"
    /// @type.node source=key type=K
    /// @resolution.name source=key target=get.key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=get.key

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const name = get(user, "name");
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source=name kind=binding target=name
/// @type.node source="get(user, \"name\")" type=string
/// @type.node source=get type=(User, "name") => string
/// @resolution.name source=get target=get
/// @resolution.call source="get(user, \"name\")" parameters=(User, "name") arguments=(provided(user) as User, provided("name") as "name") return=User["name"] kind=symbol target=get instance="get<\"name\">"
/// @generic.instantiation id="get<\"name\">" template=get arguments=("name")
/// @generic.instance id="get<\"name\">" template=get arguments=("name") dependents=(string)
/// @type.node source=user type=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @type.node source="\"name\"" type="name"

const age = get(user, "age");
/// @type.symbol symbol=age source=age type=int32
/// @resolution.pattern source=age kind=binding target=age
/// @type.node source="get(user, \"age\")" type=int32
/// @type.node source=get type=(User, "age") => int32
/// @resolution.name source=get target=get
/// @resolution.call source="get(user, \"age\")" parameters=(User, "age") arguments=(provided(user) as User, provided("age") as "age") return=User["age"] kind=symbol target=get instance="get<\"age\">"
/// @generic.instantiation id="get<\"age\">" template=get arguments=("age")
/// @generic.instance id="get<\"age\">" template=get arguments=("age") dependents=(int32)
/// @type.node source=user type=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @type.node source="\"age\"" type="age"

name satisfies string;
/// @type.node source="name satisfies string" type=string
/// @type.node source=name type=string
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="immutable"
/// @resolution.access source=name root=name

age satisfies int32;
/// @type.node source="age satisfies int32" type=int32
/// @type.node source=age type=int32
/// @resolution.name source=age target=age
/// @resolution.place source=age placement="local" lifetime="static" access="immutable"
/// @resolution.access source=age root=age
"#,
    );
}

/// An indexed access on an optional field includes undefined.
#[test]
fn test_indexed_access_optional_field_includes_undefined() {
    let session = TestSession::single(
        r#"
type User = { name?: string };
type Name = User["name"];

declare const name: Name;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name?: string };
type Name = User["name"];

declare const name: Name;

=== dir ===
type User = { name?: string };
/// @type.symbol symbol=User source="type User = { name?: string }" type={ name?: string }
/// @definition.type symbol=User source="type User = { name?: string }" value={ name?: string }
/// @type.symbol symbol=User.name source="name?: string" type=string

type Name = User["name"];
/// @type.symbol symbol=Name source="type Name = User[\"name\"]" type=string | undefined
/// @definition.type symbol=Name source="type Name = User[\"name\"]" value=User["name"]
/// @resolution.name source=User target=User

declare const name: Name;
/// @type.symbol symbol=name source=name type=Name
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=Name target=Name
"#,
    );
}

/// An indexed access projects the value type of an index signature.
#[test]
fn test_indexed_access_projects_index_signature_value() {
    let session = TestSession::single(
        r#"
type Bag = { readonly [key: string]: int32 };
type Value = Bag["name"];

declare const value: Value;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: string]: int32 };
type Value = Bag["name"];

declare const value: Value;

=== dir ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

type Value = Bag["name"];
/// @type.symbol symbol=Value source="type Value = Bag[\"name\"]" type=int32
/// @definition.type symbol=Value source="type Value = Bag[\"name\"]" value=Bag["name"]
/// @resolution.name source=Bag target=Bag

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
    );
}

/// A keyof over tuples and arrays reduces to their position keys.
#[test]
fn test_keyof_reduces_tuple_and_array_keys() {
    let session = TestSession::single(
        r#"
type Pair = keyof (string, int32);
type Open = keyof [int32];
type Fixed = keyof [int32; 3];

declare const pair: Pair;
declare const open: Open;
declare const fixed: Fixed;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair = keyof (string, int32);
type Open = keyof [int32];
type Fixed = keyof [int32; 3];

declare const pair: Pair;
declare const open: Open;
declare const fixed: Fixed;

=== dir ===
type Pair = keyof (string, int32);
/// @type.symbol symbol=Pair source="type Pair = keyof (string, int32)" type=0 | 1
/// @definition.type symbol=Pair source="type Pair = keyof (string, int32)" value=keyof (string, int32)

type Open = keyof [int32];
/// @type.symbol symbol=Open source="type Open = keyof [int32]" type=usize
/// @definition.type symbol=Open source="type Open = keyof [int32]" value=keyof Slice<int32>

type Fixed = keyof [int32; 3];
/// @type.symbol symbol=Fixed source="type Fixed = keyof [int32; 3]" type=0 | 1 | 2
/// @definition.type symbol=Fixed source="type Fixed = keyof [int32; 3]" value=keyof FixedArray<int32, 3>

declare const pair: Pair;
/// @type.symbol symbol=pair source=pair type=Pair
/// @resolution.pattern source=pair kind=binding target=pair
/// @resolution.name source=Pair target=Pair

declare const open: Open;
/// @type.symbol symbol=open source=open type=Open
/// @resolution.pattern source=open kind=binding target=open
/// @resolution.name source=Open target=Open

declare const fixed: Fixed;
/// @type.symbol symbol=fixed source=fixed type=Fixed
/// @resolution.pattern source=fixed kind=binding target=fixed
/// @resolution.name source=Fixed target=Fixed
"#,
    );
}
