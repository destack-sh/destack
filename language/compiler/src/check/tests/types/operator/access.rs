use crate::tests::{DirRows, TestSession};

#[test]
fn test_indexed_access_projects_object_property_type() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Name = User["name"];

declare const name: Name;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Name = User["name"];

declare const name: Name;

=== checked ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }

type Name = User["name"];
/// @type.symbol symbol=Name source="type Name = User[\"name\"]" type=User["name"] reduced=string
/// @definition.type symbol=Name source="type Name = User[\"name\"]" value=User["name"] reduced=string
/// @resolution.name source=User target=User

declare const name: Name;
/// @type.symbol symbol=name source=name type=Name reduced=string
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=Name target=Name
"#,
    );
}

#[test]
fn test_indexed_access_projects_usize_object_key() {
    let session = TestSession::single(
        r#"
type Pair = { 0: string; 1: int32 };
type Right = Pair[1];

declare const value: Right;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair = { 0: string; 1: int32 };
type Right = Pair[1];

declare const value: Right;

=== checked ===
type Pair = { 0: string; 1: int32 };
/// @type.symbol symbol=Pair source="type Pair = { 0: string; 1: int32 }" type={ 0: string; 1: int32 }
/// @definition.type symbol=Pair source="type Pair = { 0: string; 1: int32 }" value={ 0: string; 1: int32 }

type Right = Pair[1];
/// @type.symbol symbol=Right source="type Right = Pair[1]" type=Pair[1] reduced=int32
/// @definition.type symbol=Right source="type Right = Pair[1]" value=Pair[1] reduced=int32
/// @resolution.name source=Pair target=Pair

declare const value: Right;
/// @type.symbol symbol=value source=value type=Right reduced=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Right target=Right
"#,
    );
}

#[test]
fn test_indexed_access_rejects_missing_usize_object_key() {
    let session = TestSession::single(
        r#"
type ObjectLike = { label: string };
type Missing = ObjectLike[5];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type ObjectLike = { label: string };
type Missing = ObjectLike[5];

=== checked ===
type ObjectLike = { label: string };
/// @type.symbol symbol=ObjectLike source="type ObjectLike = { label: string }" type={ label: string }
/// @definition.type symbol=ObjectLike source="type ObjectLike = { label: string }" value={ label: string }

type Missing = ObjectLike[5];
/// @type.symbol symbol=Missing source="type Missing = ObjectLike[5]" type=ObjectLike[5] reduced=<error>
/// @definition.type symbol=Missing source="type Missing = ObjectLike[5]" value=ObjectLike[5] reduced=<error>
/// @resolution.name source=ObjectLike target=ObjectLike
"#,
        r#"
/// @diagnostic.error id=invalid-index-key message="type '{ label: string }' cannot be indexed by type '5'"
/// @diagnostic.label line=3 column=16 span="ObjectLike[5]" line_source="type Missing = ObjectLike[5];"
"#,
    );
}

#[test]
fn test_indexed_access_rejects_non_indexable_receiver() {
    let session = TestSession::single(
        r#"
type Missing = int32["name"];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Missing = int32["name"];

=== checked ===
type Missing = int32["name"];
/// @type.symbol symbol=Missing source="type Missing = int32[\"name\"]" type=int32["name"] reduced=<error>
/// @definition.type symbol=Missing source="type Missing = int32[\"name\"]" value=int32["name"] reduced=<error>
"#,
        r#"
/// @diagnostic.error id=invalid-index-receiver message="type 'int32' cannot be indexed"
/// @diagnostic.label line=2 column=16 span="int32[\"name\"]" line_source="type Missing = int32[\"name\"];"
"#,
    );
}

#[test]
fn test_indexed_access_projects_key_union() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Value = User["name" | "age"];

declare const value: Value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Value = User["name" | "age"];

declare const value: Value;

=== checked ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }

type Value = User["name" | "age"];
/// @type.symbol symbol=Value source="type Value = User[\"name\" | \"age\"]" type=User["name" | "age"] reduced=string | int32
/// @definition.type symbol=Value source="type Value = User[\"name\" | \"age\"]" value=User["name" | "age"] reduced=string | int32
/// @resolution.name source=User target=User

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value reduced=string | int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_indexed_access_key_union_rejects_unselected_value() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Value = User["name" | "age"];

const bad: Value = true;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Value = User["name" | "age"];

const bad: Value = true;

=== checked ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }

type Value = User["name" | "age"];
/// @type.symbol symbol=Value source="type Value = User[\"name\" | \"age\"]" type=User["name" | "age"] reduced=string | int32
/// @definition.type symbol=Value source="type Value = User[\"name\" | \"age\"]" value=User["name" | "age"] reduced=string | int32
/// @resolution.name source=User target=User

const bad: Value = true;
/// @type.symbol symbol=bad source=bad type=Value reduced=string | int32
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Left = { kind: "left"; value: int32 };
type Right = { kind: "right"; value: string };
type Value = (Left | Right)["value"];

const number: Value = 1 as int32 | string;
const text: Value = "hello" as int32 | string;

=== checked ===
type Left = { kind: "left"; value: int32 };
/// @type.symbol symbol=Left source="type Left = { kind: \"left\"; value: int32 }" type={ kind: "left"; value: int32 }
/// @definition.type symbol=Left source="type Left = { kind: \"left\"; value: int32 }" value={ kind: "left"; value: int32 }

type Right = { kind: "right"; value: string };
/// @type.symbol symbol=Right source="type Right = { kind: \"right\"; value: string }" type={ kind: "right"; value: string }
/// @definition.type symbol=Right source="type Right = { kind: \"right\"; value: string }" value={ kind: "right"; value: string }

type Value = (Left | Right)["value"];
/// @type.symbol symbol=Value source="type Value = (Left | Right)[\"value\"]" type=Left | Right["value"] reduced=int32 | string
/// @definition.type symbol=Value source="type Value = (Left | Right)[\"value\"]" value=Left | Right["value"] reduced=int32 | string
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

const number: Value = 1;
/// @type.symbol symbol=number source=number type=Value reduced=int32 | string
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=Value target=Value

const text: Value = "hello";
/// @type.symbol symbol=text source=text type=Value reduced=int32 | string
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=Value target=Value
"#,
    );
}

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
type Value = Input["value"];

const missing: Value = undefined as float64 | undefined | string;
const number: Value = 1 as float64 | undefined | string;
const text: Value = "hello" as float64 | undefined | string;

=== checked ===
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
/// @type.symbol symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" type={ kind: "a"; value?: float64 } | { kind: "b"; value: string }
/// @definition.type symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" value={ kind: "a"; value?: float64 } | { kind: "b"; value: string }

type Value = Input["value"];
/// @type.symbol symbol=Value source="type Value = Input[\"value\"]" type=Input["value"] reduced=float64 | undefined | string
/// @definition.type symbol=Value source="type Value = Input[\"value\"]" value=Input["value"] reduced=float64 | undefined | string
/// @resolution.name source=Input target=Input

const missing: Value = undefined;
/// @type.symbol symbol=missing source=missing type=Value reduced=float64 | undefined | string
/// @resolution.pattern source=missing kind=binding target=missing
/// @resolution.name source=Value target=Value

const number: Value = 1;
/// @type.symbol symbol=number source=number type=Value reduced=float64 | undefined | string
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=Value target=Value

const text: Value = "hello";
/// @type.symbol symbol=text source=text type=Value reduced=float64 | undefined | string
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_indexed_access_optional_union_member_rejects_unrelated_value() {
    let session = TestSession::single(
        r#"
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
type Value = Input["value"];

const bad: Value = true;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
type Value = Input["value"];

const bad: Value = true;

=== checked ===
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
/// @type.symbol symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" type={ kind: "a"; value?: float64 } | { kind: "b"; value: string }
/// @definition.type symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" value={ kind: "a"; value?: float64 } | { kind: "b"; value: string }

type Value = Input["value"];
/// @type.symbol symbol=Value source="type Value = Input[\"value\"]" type=Input["value"] reduced=float64 | undefined | string
/// @definition.type symbol=Value source="type Value = Input[\"value\"]" value=Input["value"] reduced=float64 | undefined | string
/// @resolution.name source=Input target=Input

const bad: Value = true;
/// @type.symbol symbol=bad source=bad type=Value reduced=float64 | undefined | string
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

#[test]
fn test_indexed_access_rejects_missing_object_key() {
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
=== annotated ===
type User = { name: string; age: int32 };
type Missing = User["missing"];

=== checked ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }

type Missing = User["missing"];
/// @type.symbol symbol=Missing source="type Missing = User[\"missing\"]" type=User["missing"] reduced=<error>
/// @definition.type symbol=Missing source="type Missing = User[\"missing\"]" value=User["missing"] reduced=<error>
/// @resolution.name source=User target=User
"#,
        r#"
/// @diagnostic.error id=invalid-index-key message="type '{ name: string; age: int32 }' cannot be indexed by type '\"missing\"'"
/// @diagnostic.label line=3 column=16 span="User[\"missing\"]" line_source="type Missing = User[\"missing\"];"
"#,
    );
}

#[test]
fn test_indexed_access_projects_tuple_position() {
    let session = TestSession::single(
        r#"
type Pair = (string, int32);
type First = Pair[0];

declare const first: First;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair = (string, int32);
type First = Pair[0];

declare const first: First;

=== checked ===
type Pair = (string, int32);
/// @type.symbol symbol=Pair source="type Pair = (string, int32)" type=(string, int32)
/// @definition.type symbol=Pair source="type Pair = (string, int32)" value=(string, int32)

type First = Pair[0];
/// @type.symbol symbol=First source="type First = Pair[0]" type=Pair[0] reduced=string
/// @definition.type symbol=First source="type First = Pair[0]" value=Pair[0] reduced=string
/// @resolution.name source=Pair target=Pair

declare const first: First;
/// @type.symbol symbol=first source=first type=First reduced=string
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=First target=First
"#,
    );
}

#[test]
fn test_indexed_access_projects_dynamic_array_element() {
    let session = TestSession::single(
        r#"
type Element<T: string[]> = T[usize];
type Value = Element<string[]>;

declare const value: Value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Element<T: string[]> = T[usize];
type Value = Element<string[]>;

declare const value: Value;

=== checked ===
type Element<T: string[]> = T[usize];
/// @generic.template symbol=Element parameters=(T: Array<string>)
/// @type.symbol symbol=Element source="type Element<T: string[]> = T[usize]" type=T[usize]
/// @definition.type symbol=Element source="type Element<T: string[]> = T[usize]" template=(T: Array<string>) value=T[usize]
/// @type.symbol symbol=Element.T source="T: string[]" type=T
/// @resolution.name source=T target=Element.T

type Value = Element<string[]>;
/// @type.symbol symbol=Value source="type Value = Element<string[]>" type=Element<Array<string>> reduced=string
/// @definition.type symbol=Value source="type Value = Element<string[]>" value=Element<Array<string>> reduced=string
/// @resolution.name source=Element target=Element

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value reduced=string
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

/// @generic.instance id=Element<Array<string>> template=Element arguments=(Array<string>)
"#,
    );
}

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type ValueAt<T, K: keyof T> = T[K];
type User = { name: string; age: int32 };
type Name = ValueAt<User, "name">;

declare const name: Name;

=== checked ===
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

type Name = ValueAt<User, "name">;
/// @type.symbol symbol=Name source="type Name = ValueAt<User, \"name\">" type=ValueAt<User, "name"> reduced=string
/// @definition.type symbol=Name source="type Name = ValueAt<User, \"name\">" value=ValueAt<User, "name"> reduced=string
/// @resolution.name source=ValueAt target=ValueAt
/// @resolution.name source=User target=User

declare const name: Name;
/// @type.symbol symbol=name source=name type=Name reduced=string
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=Name target=Name

/// @generic.instance id="ValueAt<User, \"name\">" template=ValueAt arguments=(User, "name")
"#,
    );
}

#[test]
fn test_generic_indexed_access_rejects_unconstrained_key() {
    let session = TestSession::single(
        r#"
type ValueAt<T, K> = T[K];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type ValueAt<T, K> = T[K];

=== checked ===
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type User = {
    readonly name: string;
    readonly age: int32;
};

function get<K: keyof User>(user: User, key: K): User[K] {
    return user[key as "name" | "age"];
}

declare const user: User;
const name: User["name"] = get<"name">(user, "name");
const age: User["age"] = get<"age">(user, "age");

name satisfies string;
age satisfies int32;

=== checked ===
type User = {
/// @type.symbol symbol=User type={ readonly name: string; readonly age: int32 }
/// @definition.type symbol=User value={ readonly name: string; readonly age: int32 }

    readonly name: string;
    readonly age: int32;
};

function get<K: keyof User>(user: User, key: K): User[K] {
/// @generic.template symbol=get parameters=(K: keyof User)
/// @type.symbol symbol=get type=<K: keyof User>(User, K) => User[K]
/// @type.symbol symbol=get.K source="K: keyof User" type=K
/// @resolution.name source=User target=User
/// @type.symbol symbol=get.user source="user: User" type=User reduced={ readonly name: string; readonly age: int32 }
/// @resolution.name source=User target=User
/// @type.symbol symbol=get.key source="key: K" type=K
/// @resolution.name source=K target=get.K
/// @resolution.name source=User target=User
/// @resolution.name source=K target=get.K

    return user[key];
    /// @type.node source=user type=User reduced={ readonly name: string; readonly age: int32 }
    /// @type.node source=user[key] type={ readonly name: string; readonly age: int32 }[K]
    /// @resolution.name source=user target=get.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=get.user
    /// @resolution.place source=user[key] placement="local" lifetime="frame" access="exclusive"
    /// @resolution.subscript source=user[key] type={ readonly name: string; readonly age: int32 }[K] kind=member target="receiver={ readonly name: string; readonly age: int32 }, target=index(keyof { readonly name: string; readonly age: int32 }), type={ readonly name: string; readonly age: int32 }[K]"
    /// @type.node source=key type=K
    /// @resolution.name source=key target=get.key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=get.key

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User reduced={ readonly name: string; readonly age: int32 }
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const name = get(user, "name");
/// @type.symbol symbol=name source=name type=User["name"] reduced=string
/// @resolution.pattern source=name kind=binding target=name
/// @type.node source="get(user, \"name\")" type=User["name"] reduced=string
/// @type.node source=get type=(User, "name") => User["name"] reduced=(User, "name") => string
/// @resolution.name source=get target=get
/// @resolution.call source="get(user, \"name\")" parameters=(User, "name") arguments=(provided(user) as User, provided("name") as "name") return=User["name"] kind=symbol target=get instance="get<\"name\">"
/// @generic.instance source="get(user, \"name\")" id="get<\"name\">"
/// @type.node source=user type=User reduced={ readonly name: string; readonly age: int32 }
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
/// @type.node source="\"name\"" type="name"

const age = get(user, "age");
/// @type.symbol symbol=age source=age type=User["age"] reduced=int32
/// @resolution.pattern source=age kind=binding target=age
/// @type.node source="get(user, \"age\")" type=User["age"] reduced=int32
/// @type.node source=get type=(User, "age") => User["age"] reduced=(User, "age") => int32
/// @resolution.name source=get target=get
/// @resolution.call source="get(user, \"age\")" parameters=(User, "age") arguments=(provided(user) as User, provided("age") as "age") return=User["age"] kind=symbol target=get instance="get<\"age\">"
/// @generic.instance source="get(user, \"age\")" id="get<\"age\">"
/// @type.node source=user type=User reduced={ readonly name: string; readonly age: int32 }
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
/// @type.node source="\"age\"" type="age"

name satisfies string;
/// @type.node source="name satisfies string" type=User["name"] reduced=string
/// @type.node source=name type=User["name"] reduced=string
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=name root=name

age satisfies int32;
/// @type.node source="age satisfies int32" type=User["age"] reduced=int32
/// @type.node source=age type=User["age"] reduced=int32
/// @resolution.name source=age target=age
/// @resolution.place source=age placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=age root=age

/// @generic.instance id="get<\"age\">" template=get arguments=("age")
/// @generic.instance id="get<\"name\">" template=get arguments=("name")
"#,
    );
}

#[test]
fn test_indexed_access_optional_field_includes_undefined() {
    let session = TestSession::single(
        r#"
type User = { name?: string };
type Name = User["name"];

declare const name: Name;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name?: string };
type Name = User["name"];

declare const name: Name;

=== checked ===
type User = { name?: string };
/// @type.symbol symbol=User source="type User = { name?: string }" type={ name?: string }
/// @definition.type symbol=User source="type User = { name?: string }" value={ name?: string }

type Name = User["name"];
/// @type.symbol symbol=Name source="type Name = User[\"name\"]" type=User["name"] reduced=string | undefined
/// @definition.type symbol=Name source="type Name = User[\"name\"]" value=User["name"] reduced=string | undefined
/// @resolution.name source=User target=User

declare const name: Name;
/// @type.symbol symbol=name source=name type=Name reduced=string | undefined
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=Name target=Name
"#,
    );
}

#[test]
fn test_indexed_access_projects_unique_symbol_key() {
    let session = TestSession::single(
        r#"
declare const token: unique symbol;

type TokenBox = { readonly [token]: int32 };
type Value = TokenBox[token];

declare const value: Value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const token: unique symbol;

type TokenBox = { readonly [token]: int32 };
type Value = TokenBox[token];

declare const value: Value;

=== checked ===
declare const token: unique symbol;
/// @type.symbol symbol=token source=token type=unique symbol
/// @resolution.pattern source=token kind=binding target=token

type TokenBox = { readonly [token]: int32 };
/// @type.symbol symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" type={ readonly [token]: int32 }
/// @definition.type symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" value={ readonly [token]: int32 }

type Value = TokenBox[token];
/// @type.symbol symbol=Value source="type Value = TokenBox[token]" type=TokenBox[token] reduced=int32
/// @definition.type symbol=Value source="type Value = TokenBox[token]" value=TokenBox[token] reduced=int32
/// @resolution.name source=TokenBox target=TokenBox
/// @resolution.name source=token target=token

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value reduced=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_indexed_access_projects_imported_unique_symbol_key() {
    let session = TestSession::builder()
        .module(
            "keys.ds",
            r#"
export declare const token: unique symbol;
export type TokenBox = { readonly [token]: int32 };
"#,
        )
        .module(
            "main.ds",
            r#"
import { token, TokenBox } from "./keys.ds";

type Value = TokenBox[token];
declare const value: Value;
"#,
        )
        .build();

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { TokenBox, token } from "./keys.ds";

type Value = TokenBox[token];
declare const value: Value;

=== checked ===
import { token, TokenBox } from "./keys.ds";

type Value = TokenBox[token];
/// @type.symbol symbol=Value source="type Value = TokenBox[token]" type=keys.TokenBox[keys.token] reduced=int32
/// @definition.type symbol=Value source="type Value = TokenBox[token]" value=keys.TokenBox[keys.token] reduced=int32
/// @resolution.name source=TokenBox target=keys.TokenBox
/// @resolution.name source=token target=keys.token

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value reduced=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_indexed_access_projects_index_signature_value() {
    let session = TestSession::single(
        r#"
type Bag = { readonly [key: string]: int32 };
type Value = Bag["name"];

declare const value: Value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: string]: int32 };
type Value = Bag["name"];

declare const value: Value;

=== checked ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

type Value = Bag["name"];
/// @type.symbol symbol=Value source="type Value = Bag[\"name\"]" type=Bag["name"] reduced=int32
/// @definition.type symbol=Value source="type Value = Bag[\"name\"]" value=Bag["name"] reduced=int32
/// @resolution.name source=Bag target=Bag

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value reduced=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
    );
}

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

    session.assert_dir_checked(
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

=== checked ===
type Pair = keyof (string, int32);
/// @type.symbol symbol=Pair source="type Pair = keyof (string, int32)" type=keyof (string, int32) reduced=0 | 1
/// @definition.type symbol=Pair source="type Pair = keyof (string, int32)" value=keyof (string, int32) reduced=0 | 1

type Open = keyof [int32];
/// @type.symbol symbol=Open source="type Open = keyof [int32]" type=keyof Slice<int32> reduced=usize
/// @definition.type symbol=Open source="type Open = keyof [int32]" value=keyof Slice<int32> reduced=usize

type Fixed = keyof [int32; 3];
/// @type.symbol symbol=Fixed source="type Fixed = keyof [int32; 3]" type=keyof FixedArray<int32, 3> reduced=0 | 1 | 2
/// @definition.type symbol=Fixed source="type Fixed = keyof [int32; 3]" value=keyof FixedArray<int32, 3> reduced=0 | 1 | 2

declare const pair: Pair;
/// @type.symbol symbol=pair source=pair type=Pair reduced=0 | 1
/// @resolution.pattern source=pair kind=binding target=pair
/// @resolution.name source=Pair target=Pair

declare const open: Open;
/// @type.symbol symbol=open source=open type=Open reduced=usize
/// @resolution.pattern source=open kind=binding target=open
/// @resolution.name source=Open target=Open

declare const fixed: Fixed;
/// @type.symbol symbol=fixed source=fixed type=Fixed reduced=0 | 1 | 2
/// @resolution.pattern source=fixed kind=binding target=fixed
/// @resolution.name source=Fixed target=Fixed
"#,
    );
}
