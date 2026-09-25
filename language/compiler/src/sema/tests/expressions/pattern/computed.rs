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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32 };

let { ["x"]: value } = point;

value satisfies int32;

=== dir ===
declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x source="x: int32" type=int32

let { ["x"]: value } = point;
/// @resolution.pattern source={ ["x"]: value } kind=object fields={ x: value }
/// @type.node source="\"x\"" type="x"
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point

value satisfies int32;
/// @type.node source="value satisfies int32" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const key: string;
declare const point: { x: int32 };

let { [key]: value } = point;

=== dir ===
declare const key: string;
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key

declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x source="x: int32" type=int32

let { [key]: value } = point;
/// @resolution.pattern source={ [key]: value } kind=object fields={}
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @resolution.place source=key placement="local" lifetime="static" access="immutable"
/// @resolution.access source=key root=key
/// @type.symbol symbol=value source=value type=<error>
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point
"#,
        r#"
/// @diagnostic.error id=computed-pattern-key-not-valid message="computed pattern key is not valid for the source type"
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Bag = { [key: string]: int32 };

declare const key: string;
declare const bag: Bag;

let { [key]: value } = bag;

value satisfies int32 | undefined;

=== dir ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

declare const key: string;
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

let { [key]: value } = bag;
/// @resolution.pattern source={ [key]: value } kind=object fields={ subscript(member(receiver={ [key: string]: int32 }, target=index(string), type=int32 | undefined), int32 | undefined): value }
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @resolution.place source=key placement="local" lifetime="static" access="immutable"
/// @resolution.access source=key root=key
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=bag type=Bag
/// @resolution.name source=bag target=bag
/// @resolution.access source=bag root=bag

value satisfies int32 | undefined;
/// @type.node source="value satisfies int32 | undefined" type=int32 | undefined
/// @type.node source=value type=int32 | undefined
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type User = {
    readonly name: string;
    readonly age: int32;
};

function get<K: keyof User>(user: User, key: K): User[K] {
    let { [key]: value } = user;
    return value;
}

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

    let { [key]: value } = user;
    /// @resolution.pattern source={ [key]: value } kind=object fields={ subscript(member(receiver={ readonly name: string; readonly age: int32 }, target=index(keyof { readonly name: string; readonly age: int32 }), type={ readonly name: string; readonly age: int32 }[K]), { readonly name: string; readonly age: int32 }[K]): get.value }
    /// @type.node source=key type=K
    /// @resolution.name source=key target=get.key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=get.key
    /// @type.symbol symbol=get.value source=value type={ readonly name: string; readonly age: int32 }[K]
    /// @resolution.pattern source=value kind=binding target=get.value
    /// @type.node source=user type=User
    /// @resolution.name source=user target=get.user
    /// @resolution.access source=user root=get.user

    return value;
    /// @type.node source=value type={ readonly name: string; readonly age: int32 }[K]
    /// @resolution.name source=value target=get.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=get.value

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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const pair: { 0: string; 1: int32 };

let { [1]: value } = pair;

value satisfies int32;

=== dir ===
declare const pair: { 0: string; 1: int32 };
/// @type.symbol symbol=pair source=pair type={ 0: string; 1: int32 }
/// @resolution.pattern source=pair kind=binding target=pair
/// @type.symbol symbol=symbol1 source="0: string" type=string
/// @type.symbol symbol=symbol3 source="1: int32" type=int32

let { [1]: value } = pair;
/// @resolution.pattern source={ [1]: value } kind=object fields={ 1: value }
/// @type.node source=1 type=1
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=pair type={ 0: string; 1: int32 }
/// @resolution.name source=pair target=pair
/// @resolution.access source=pair root=pair

value satisfies int32;
/// @type.node source="value satisfies int32" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
"#,
    );
}
