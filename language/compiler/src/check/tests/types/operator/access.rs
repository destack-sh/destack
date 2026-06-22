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
/// @type.symbol symbol=Name source="type Name = User[\"name\"]" type=string
/// @definition.type symbol=Name source="type Name = User[\"name\"]" value=string
/// @resolution.name source=User target=User

declare const name: Name;
/// @type.symbol symbol=name source=name type=string
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
/// @type.symbol symbol=Right source="type Right = Pair[1]" type=int32
/// @definition.type symbol=Right source="type Right = Pair[1]" value=int32
/// @resolution.name source=Pair target=Pair

declare const value: Right;
/// @type.symbol symbol=value source=value type=int32
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
/// @type.symbol symbol=Missing source="type Missing = ObjectLike[5]" type=<error>
/// @definition.type symbol=Missing source="type Missing = ObjectLike[5]" value=<error>
/// @resolution.name source=ObjectLike target=ObjectLike

"#,
        r#"
/// @diagnostic.error code=EC316 message="type '{ label: string }' cannot be indexed by type '5'"
/// @diagnostic.label line=3 column=16 source="type Missing = ObjectLike[5];"
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
/// @type.symbol symbol=Missing source="type Missing = int32[\"name\"]" type=<error>
/// @definition.type symbol=Missing source="type Missing = int32[\"name\"]" value=<error>

"#,
        r#"
/// @diagnostic.error code=EC315 message="type 'int32' cannot be indexed"
/// @diagnostic.label line=2 column=1 source="type Missing = int32[\"name\"];"
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
/// @type.symbol symbol=Value source="type Value = User[\"name\" | \"age\"]" type=string | int32
/// @definition.type symbol=Value source="type Value = User[\"name\" | \"age\"]" value=string | int32
/// @resolution.name source=User target=User

declare const value: Value;
/// @type.symbol symbol=value source=value type=string | int32
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
/// @type.symbol symbol=Value source="type Value = User[\"name\" | \"age\"]" type=string | int32
/// @definition.type symbol=Value source="type Value = User[\"name\" | \"age\"]" value=string | int32
/// @resolution.name source=User target=User

const bad: Value = true;
/// @type.symbol symbol=bad source=bad type=string | int32
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'true' is not assignable to type 'Value'"
/// @diagnostic.label line=5 column=7 source="const bad: Value = true;"
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

const number: Value = 1 as Value;
const text: Value = "hello" as Value;

=== checked ===
type Left = { kind: "left"; value: int32 };
/// @type.symbol symbol=Left source="type Left = { kind: \"left\"; value: int32 }" type={ kind: "left"; value: int32 }
/// @definition.type symbol=Left source="type Left = { kind: \"left\"; value: int32 }" value={ kind: "left"; value: int32 }

type Right = { kind: "right"; value: string };
/// @type.symbol symbol=Right source="type Right = { kind: \"right\"; value: string }" type={ kind: "right"; value: string }
/// @definition.type symbol=Right source="type Right = { kind: \"right\"; value: string }" value={ kind: "right"; value: string }

type Value = (Left | Right)["value"];
/// @type.symbol symbol=Value source="type Value = (Left | Right)[\"value\"]" type=int32 | string
/// @definition.type symbol=Value source="type Value = (Left | Right)[\"value\"]" value=int32 | string
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

const number: Value = 1;
/// @type.symbol symbol=number source=number type=int32 | string
/// @resolution.name source=Value target=Value

const text: Value = "hello";
/// @type.symbol symbol=text source=text type=int32 | string
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

const missing: Value = undefined as Value;
const number: Value = 1 as Value;
const text: Value = "hello" as Value;

=== checked ===
type Input = { kind: "a"; value?: number } | { kind: "b"; value: string };
/// @type.symbol symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" type={ kind: "a"; value?: number } | { kind: "b"; value: string }
/// @definition.type symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" value={ kind: "a"; value?: number } | { kind: "b"; value: string }

type Value = Input["value"];
/// @type.symbol symbol=Value source="type Value = Input[\"value\"]" type=number | string | undefined
/// @definition.type symbol=Value source="type Value = Input[\"value\"]" value=number | string | undefined
/// @resolution.name source=Input target=Input

const missing: Value = undefined;
/// @type.symbol symbol=missing source=missing type=number | string | undefined
/// @resolution.name source=Value target=Value

const number: Value = 1;
/// @type.symbol symbol=number source=number type=number | string | undefined
/// @resolution.name source=Value target=Value

const text: Value = "hello";
/// @type.symbol symbol=text source=text type=number | string | undefined
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
/// @type.symbol symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" type={ kind: "a"; value?: number } | { kind: "b"; value: string }
/// @definition.type symbol=Input source="type Input = { kind: \"a\"; value?: number } | { kind: \"b\"; value: string }" value={ kind: "a"; value?: number } | { kind: "b"; value: string }

type Value = Input["value"];
/// @type.symbol symbol=Value source="type Value = Input[\"value\"]" type=number | string | undefined
/// @definition.type symbol=Value source="type Value = Input[\"value\"]" value=number | string | undefined
/// @resolution.name source=Input target=Input

const bad: Value = true;
/// @type.symbol symbol=bad source=bad type=number | string | undefined
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'true' is not assignable to type 'Value'"
/// @diagnostic.label line=5 column=7 source="const bad: Value = true;"
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
/// @type.symbol symbol=Missing source="type Missing = User[\"missing\"]" type=<error>
/// @definition.type symbol=Missing source="type Missing = User[\"missing\"]" value=<error>
/// @resolution.name source=User target=User

"#,
        r#"
/// @diagnostic.error code=EC316 message="type '{ name: string; age: int32 }' cannot be indexed by type '\"missing\"'"
/// @diagnostic.label line=3 column=16 source="type Missing = User[\"missing\"];"
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
/// @type.symbol symbol=First source="type First = Pair[0]" type=string
/// @definition.type symbol=First source="type First = Pair[0]" value=string
/// @resolution.name source=Pair target=Pair

declare const first: First;
/// @type.symbol symbol=first source=first type=string
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
/// @generic.template symbol=Element parameters=(T: string[])
/// @type.symbol symbol=Element source="type Element<T: string[]> = T[usize]" type=T[usize]
/// @definition.type symbol=Element source="type Element<T: string[]> = T[usize]" template=LocalGenericTemplateId(0) value=T[usize]
/// @type.symbol symbol=Element.T source=T type=T
/// @resolution.name source=T target=Element.T

type Value = Element<string[]>;
/// @type.symbol symbol=Value source="type Value = Element<string[]>" type=string
/// @definition.type symbol=Value source="type Value = Element<string[]>" value=string
/// @resolution.name source=Element target=Element

declare const value: Value;
/// @type.symbol symbol=value source=value type=string
/// @resolution.name source=Value target=Value
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
/// @definition.type symbol=ValueAt source="type ValueAt<T, K: keyof T> = T[K]" template=LocalGenericTemplateId(0) value=T[K]
/// @type.symbol symbol=ValueAt.T source=T type=T
/// @type.symbol symbol=ValueAt.K source=K type=K
/// @resolution.name source=T target=ValueAt.T
/// @resolution.name source=T target=ValueAt.T
/// @resolution.name source=K target=ValueAt.K

type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }

type Name = ValueAt<User, "name">;
/// @type.symbol symbol=Name source="type Name = ValueAt<User, \"name\">" type=string
/// @definition.type symbol=Name source="type Name = ValueAt<User, \"name\">" value=string
/// @resolution.name source=ValueAt target=ValueAt
/// @resolution.name source=User target=User

declare const name: Name;
/// @type.symbol symbol=name source=name type=string
/// @resolution.name source=Name target=Name
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
/// @definition.type symbol=ValueAt source="type ValueAt<T, K> = T[K]" template=LocalGenericTemplateId(0) value=T[K]
/// @type.symbol symbol=ValueAt.T source=T type=T
/// @type.symbol symbol=ValueAt.K source=K type=K
/// @resolution.name source=T target=ValueAt.T
/// @resolution.name source=K target=ValueAt.K

"#,
        r#"
/// @diagnostic.error code=EC316 message="type 'T' cannot be indexed by type 'K'"
/// @diagnostic.label line=2 column=1 source="type ValueAt<T, K> = T[K];"
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

function get<T0: User, K: keyof User>(user: T0, key: K): User[K] {
    return user[key];
}

declare const user: User;
const name: string = get<User, "name">(user, "name");
const age: int32 = get<User, "age">(user, "age");

name satisfies string;
age satisfies int32;

=== checked ===
type User = {
/// @type.symbol symbol=User source="type User = {\n    readonly name: string;\n    readonly age: int32;\n}" type={ readonly name: string; readonly age: int32 }
/// @definition.type symbol=User source="type User = {\n    readonly name: string;\n    readonly age: int32;\n}" value={ readonly name: string; readonly age: int32 }

    readonly name: string;
    readonly age: int32;
};

function get<K: keyof User>(user: User, key: K): User[K] {
/// @generic.template symbol=get parameters=(T0: User, K: keyof User)
/// @type.symbol symbol=get type=<get.T0: User, K: keyof User>(get.T0, K) => User[K]
/// @type.symbol symbol=user type=get.T0
/// @type.symbol symbol=key type=K
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User
/// @resolution.name source=K target=get.K

    return user[key];
    /// @resolution.name source=user target=user
    /// @resolution.name source=key target=key
    /// @type.node source=user type=get.T0
    /// @type.node source=key type=K
    /// @type.node source=user[key] type=User[K]

}

declare const user: User;
/// @type.symbol symbol=user source=user type={ readonly name: string; readonly age: int32 }
/// @resolution.name source=User target=User

const name = get(user, "name");
/// @type.symbol symbol=name type=string
/// @resolution.name source=get target=get
/// @resolution.name source=user target=user
/// @resolution.call source="get(user, \"name\")" parameters=({ readonly name: string; readonly age: int32 }, "name") return=string kind=symbol target=get instance="get<User, \"name\">"
/// @generic.instance source="get(user, \"name\")" id="get<User, \"name\">"
/// @type.node source=get type=(User, "name") => string
/// @type.node source=user type={ readonly name: string; readonly age: int32 }
/// @type.node source="\"name\"" type="name"
/// @type.node source="get(user, \"name\")" type=string

const age = get(user, "age");
/// @type.symbol symbol=age type=int32
/// @resolution.name source=get target=get
/// @resolution.name source=user target=user
/// @resolution.call source="get(user, \"age\")" parameters=({ readonly name: string; readonly age: int32 }, "age") return=int32 kind=symbol target=get instance="get<User, \"age\">"
/// @generic.instance source="get(user, \"age\")" id="get<User, \"age\">"
/// @type.node source=get type=(User, "age") => int32
/// @type.node source=user type={ readonly name: string; readonly age: int32 }
/// @type.node source="\"age\"" type="age"
/// @type.node source="get(user, \"age\")" type=int32

name satisfies string;
/// @resolution.name source=name target=name
/// @type.node source=name type=string

age satisfies int32;
/// @resolution.name source=age target=age
/// @type.node source=age type=int32
/// @generic.instance id="get<User, \"age\">" template=get arguments=(User, "age")
/// @generic.instance id="get<User, \"name\">" template=get arguments=(User, "name")
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
/// @type.symbol symbol=Name source="type Name = User[\"name\"]" type=string | undefined
/// @definition.type symbol=Name source="type Name = User[\"name\"]" value=string | undefined
/// @resolution.name source=User target=User

declare const name: Name;
/// @type.symbol symbol=name source=name type=string | undefined
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

type TokenBox = { readonly [token]: int32 };
/// @type.symbol symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" type={ readonly [token]: int32 }
/// @definition.type symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" value={ readonly [token]: int32 }
/// @resolution.name source=token target=token

type Value = TokenBox[token];
/// @type.symbol symbol=Value source="type Value = TokenBox[token]" type=int32
/// @definition.type symbol=Value source="type Value = TokenBox[token]" value=int32
/// @resolution.name source=TokenBox target=TokenBox
/// @resolution.name source=token target=token

declare const value: Value;
/// @type.symbol symbol=value source=value type=int32
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
/// @type.symbol symbol=Value source="type Value = Bag[\"name\"]" type=int32
/// @definition.type symbol=Value source="type Value = Bag[\"name\"]" value=int32
/// @resolution.name source=Bag target=Bag

declare const value: Value;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.name source=Value target=Value
"#,
    );
}
