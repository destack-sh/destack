use crate::tests::{DirRows, TestSession};

#[test]
fn test_computed_object_pattern_accepts_static_string_key() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32 };

let { ["x"]: value } = point;

value satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32 };

let { ["x"]: value } = point;

value satisfies int32;

=== checked ===
declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }

let { ["x"]: value } = point;
/// @resolution.pattern source={ ["x"]: value } kind=object fields={ x: value }
/// @type.node source="\"x\"" type="x"
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point

value satisfies int32;
/// @type.node source="value satisfies int32" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_computed_object_pattern_rejects_unbounded_key_for_finite_shape() {
    let session = TestSession::single(
        r#"
declare const key: string;
declare const point: { x: int32 };

let { [key]: value } = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const key: string;
declare const point: { x: int32 };

let { [key]: value } = point;

=== checked ===
declare const key: string;
/// @type.symbol symbol=key source=key type=string

declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }

let { [key]: value } = point;
/// @resolution.pattern source={ [key]: value } kind=object fields={}
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC432 message="computed pattern key is not valid for the source type"
/// @diagnostic.label line=5 column=8 span="key" line_source="let { [key]: value } = point;"
"#,
    );
}

#[test]
fn test_computed_object_pattern_accepts_index_signature_key() {
    let session = TestSession::single(
        r#"
type Bag = { [key: string]: int32 };

declare const key: string;
declare const bag: Bag;

let { [key]: value } = bag;

value satisfies int32 | undefined;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Bag = { [key: string]: int32 };

declare const key: string;
declare const bag: Bag;

let { [key]: value } = bag;

value satisfies int32 | undefined;

=== checked ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

declare const key: string;
/// @type.symbol symbol=key source=key type=string

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag
/// @resolution.name source=Bag target=Bag

let { [key]: value } = bag;
/// @resolution.pattern source={ [key]: value } kind=object fields={ key: value }
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=bag type={ [key: string]: int32 }
/// @resolution.name source=bag target=bag

value satisfies int32 | undefined;
/// @type.node source="value satisfies int32 | undefined" type=int32 | undefined
/// @type.node source=value type=int32 | undefined
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_computed_object_pattern_accepts_keyof_generic_key() {
    let session = TestSession::single(
        r#"
type User = {
    readonly name: string;
    readonly age: int32;
};

function get<K: keyof User>(user: User, key: K): User[K] {
    let { [key]: value } = user;
    return value;
}
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

function get<K: keyof User, T1: User>(user: T1, key: K): User[K] {
    let { [key]: value } = user;
    return value;
}

=== checked ===
type User = {
/// @type.symbol symbol=User type={ readonly name: string; readonly age: int32 }
/// @definition.type symbol=User value={ readonly name: string; readonly age: int32 }

    readonly name: string;
    readonly age: int32;
};

function get<K: keyof User>(user: User, key: K): User[K] {
/// @generic.template symbol=get parameters=(K: keyof User, T1: User)
/// @type.symbol symbol=get type=<K: keyof User, get.T1: User>(get.T1, K) => User[K]
/// @type.symbol symbol=get.K source="K: keyof User" type=K
/// @resolution.name source=User target=User
/// @type.symbol symbol=get.user source="user: User" type=get.T1
/// @resolution.name source=User target=User
/// @type.symbol symbol=get.key source="key: K" type=K
/// @resolution.name source=K target=get.K
/// @resolution.name source=User target=User
/// @resolution.name source=K target=get.K

    let { [key]: value } = user;
    /// @resolution.pattern source={ [key]: value } kind=object fields={ key: get.value }
    /// @type.node source=key type=K
    /// @resolution.name source=key target=get.key
    /// @type.symbol symbol=get.value source=value type=User[K]
    /// @resolution.pattern source=value kind=binding target=get.value
    /// @type.node source=user type=get.T1
    /// @resolution.name source=user target=get.user

    return value;
    /// @type.node source=value type=User[K]
    /// @resolution.name source=value target=get.value

}
"#,
    );
}

#[test]
fn test_computed_object_pattern_accepts_static_numeric_key() {
    let session = TestSession::single(
        r#"
declare const pair: { 0: string; 1: int32 };

let { [1]: value } = pair;

value satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const pair: { 0: string; 1: int32 };

let { [1]: value } = pair;

value satisfies int32;

=== checked ===
declare const pair: { 0: string; 1: int32 };
/// @type.symbol symbol=pair source=pair type={ 0: string; 1: int32 }

let { [1]: value } = pair;
/// @resolution.pattern source={ [1]: value } kind=object fields={ 1: value }
/// @type.node source=1 type=1
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=pair type={ 0: string; 1: int32 }
/// @resolution.name source=pair target=pair

value satisfies int32;
/// @type.node source="value satisfies int32" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_computed_object_pattern_accepts_unique_symbol_key() {
    let session = TestSession::single(
        r#"
declare const token: unique symbol;
declare const box: { readonly [token]: string };

let { [token]: value } = box;

value satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const token: unique symbol;
declare const box: { readonly [token]: string };

let { [token]: value } = box;

value satisfies string;

=== checked ===
declare const token: unique symbol;
/// @type.symbol symbol=token source=token type=unique symbol

declare const box: { readonly [token]: string };
/// @type.symbol symbol=box source=box type={ readonly [token]: string }
/// @resolution.name source=token target=token

let { [token]: value } = box;
/// @resolution.pattern source={ [token]: value } kind=object fields={ token: value }
/// @type.node source=token type=token
/// @resolution.name source=token target=token
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=box type={ readonly [token]: string }
/// @resolution.name source=box target=box

value satisfies string;
/// @type.node source="value satisfies string" type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_computed_object_pattern_accepts_registry_symbol_key() {
    let session = TestSession::single(
        r#"
declare const box: { readonly [Symbol.for("token")]: string };

let { [Symbol.for("token")]: value } = box;

value satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const box: { readonly [Symbol.for("token")]: string };

let { [Symbol.for("token")]: value } = box;

value satisfies string;

=== checked ===
declare const box: { readonly [Symbol.for("token")]: string };
/// @type.symbol symbol=box source=box type={ readonly [Symbol.for("token")]: string }
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.for receiver=types.symbol.Symbol kind=symbol target=types.symbol.Symbol.for
/// @resolution.call source="Symbol.for(\"token\")" parameters=(string) arguments=(provided("token") as string) return=symbol kind=symbol target=types.symbol.Symbol.for receiver=types.symbol.Symbol

let { [Symbol.for("token")]: value } = box;
/// @resolution.pattern source={ [Symbol.for("token")]: value } kind=object fields={ Symbol.for("token"): value }
/// @type.node source="Symbol.for(\"token\")" type=symbol
/// @type.node source=Symbol type=types.symbol.Symbol
/// @type.node source=Symbol.for type=(string) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.for receiver=types.symbol.Symbol kind=symbol target=types.symbol.Symbol.for
/// @resolution.call source="Symbol.for(\"token\")" parameters=(string) arguments=(provided("token") as string) return=symbol kind=symbol target=types.symbol.Symbol.for receiver=types.symbol.Symbol
/// @type.node source="\"token\"" type="token"
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=box type={ readonly [Symbol.for("token")]: string }
/// @resolution.name source=box target=box

value satisfies string;
/// @type.node source="value satisfies string" type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value
"#,
    );
}
