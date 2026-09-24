use crate::tests::{DirRows, TestSession};

#[test]
fn test_in_returns_boolean_for_known_property() {
    let session = TestSession::single(
        r#"
const point = { x: 1, y: 2 };

const hasX = "x" in point;
hasX satisfies boolean;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const point: { x: int64; y: int64 } = { x: 1, y: 2 };

const hasX: boolean = "x" in point;
hasX satisfies boolean;

=== dir ===
const point = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: int64; y: int64 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source={ x: 1, y: 2 } type={ x: int64; y: int64 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const hasX = "x" in point;
/// @type.symbol symbol=hasX source=hasX type=boolean
/// @resolution.pattern source=hasX kind=binding target=hasX
/// @type.node source="\"x\" in point" type=boolean
/// @type.node source="\"x\"" type="x"
/// @resolution.guard source="\"x\" in point" kind=in key_type="x" receiver={ x: int64; y: int64 } predicate="membership({ x: int64; y: int64 }, x)" narrowed={ x: int64; y: int64 }
/// @type.node source=point type={ x: int64; y: int64 }
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point

hasX satisfies boolean;
/// @type.node source="hasX satisfies boolean" type=boolean
/// @type.node source=hasX type=boolean
/// @resolution.name source=hasX target=hasX
/// @resolution.place source=hasX placement="local" lifetime="static" access="immutable"
/// @resolution.access source=hasX root=hasX
"#,
    );
}

#[test]
fn test_in_returns_boolean_for_missing_property() {
    let session = TestSession::single(
        r#"
const point = { x: 1, y: 2 };

const hasName = "name" in point;
hasName satisfies boolean;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const point: { x: int64; y: int64 } = { x: 1, y: 2 };

const hasName: boolean = "name" in point;
hasName satisfies boolean;

=== dir ===
const point = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: int64; y: int64 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source={ x: 1, y: 2 } type={ x: int64; y: int64 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const hasName = "name" in point;
/// @type.symbol symbol=hasName source=hasName type=boolean
/// @resolution.pattern source=hasName kind=binding target=hasName
/// @type.node source="\"name\" in point" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in point" kind=in key_type="name" receiver={ x: int64; y: int64 } predicate="membership({ x: int64; y: int64 }, name)" narrowed=never
/// @type.node source=point type={ x: int64; y: int64 }
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point

hasName satisfies boolean;
/// @type.node source="hasName satisfies boolean" type=boolean
/// @type.node source=hasName type=boolean
/// @resolution.name source=hasName target=hasName
/// @resolution.place source=hasName placement="local" lifetime="static" access="immutable"
/// @resolution.access source=hasName root=hasName
"#,
    );
}

#[test]
fn test_in_uses_structural_membership_for_nominal_receiver() {
    let session = TestSession::single(
        r#"
class Bag {
    has(key: &readonly string): boolean {
        return true;
    }
}

declare const bag: Bag;

const found = "name" in bag;
found satisfies boolean;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Bag {
    has(key: &'a readonly string): boolean {
        return true;
    }
}

declare const bag: Bag;

const found: boolean = "name" in bag;
found satisfies boolean;

=== dir ===
class Bag {
/// @type.symbol symbol=Bag type=typeof Bag
/// @definition.class symbol=Bag
/// @definition.method symbol=Bag.has slot=has type=<Bag.has.'a>(this: Bag, &Bag.has.'a readonly string) => boolean

    has(key: &readonly string): boolean {
    /// @generic.template symbol=Bag.has parameters=('a)
    /// @type.symbol symbol=Bag.has type=<Bag.has.'a>(this: Bag, &Bag.has.'a readonly string) => boolean
    /// @type.symbol symbol=Bag.has.this type=Bag
    /// @type.symbol symbol=Bag.has.key source="key: &readonly string" type=&Bag.has.'a readonly string

        return true;
        /// @type.node source=true type=true

    }
}

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

const found = "name" in bag;
/// @type.symbol symbol=found source=found type=boolean
/// @resolution.pattern source=found kind=binding target=found
/// @type.node source="\"name\" in bag" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in bag" kind=in key_type="name" receiver=Bag predicate="membership(Bag, name)" narrowed=Bag & { readonly name: unknown }
/// @type.node source=bag type=Bag
/// @resolution.name source=bag target=bag
/// @resolution.place source=bag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bag root=bag

found satisfies boolean;
/// @type.node source="found satisfies boolean" type=boolean
/// @type.node source=found type=boolean
/// @resolution.name source=found target=found
/// @resolution.place source=found placement="local" lifetime="static" access="immutable"
/// @resolution.access source=found root=found
"#,
    );
}

#[test]
fn test_in_accepts_nominal_receiver_field() {
    let session = TestSession::single(
        r#"
declare class User {
    name: string;
}

declare const user: User;

"name" in user;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class User {
    name: string;
}

declare const user: User;

"name" in user;

=== dir ===
declare class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

"name" in user;
/// @type.node source="\"name\" in user" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in user" kind=in key_type="name" receiver=User predicate="membership(User, name)" narrowed=User
/// @type.node source=user type=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
"#,
    );
}

#[test]
fn test_in_narrows_object_union_by_property() {
    let session = TestSession::single(
        r#"
type Named = { name: string };
type Numbered = { id: int32 };

declare const value: Named | Numbered;

if ("name" in value) {
    value.name satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Named = { name: string };
type Numbered = { id: int32 };

declare const value: { name: string } | { id: int32 };

if ("name" in value) {
    value.name satisfies string;
}

=== dir ===
type Named = { name: string };
/// @type.symbol symbol=Named source="type Named = { name: string }" type={ name: string }
/// @definition.type symbol=Named source="type Named = { name: string }" value={ name: string }
/// @type.symbol symbol=Named.name source="name: string" type=string

type Numbered = { id: int32 };
/// @type.symbol symbol=Numbered source="type Numbered = { id: int32 }" type={ id: int32 }
/// @definition.type symbol=Numbered source="type Numbered = { id: int32 }" value={ id: int32 }
/// @type.symbol symbol=Numbered.id source="id: int32" type=int32

declare const value: Named | Numbered;
/// @type.symbol symbol=value source=value type={ name: string } | { id: int32 }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Named target=Named
/// @resolution.name source=Numbered target=Numbered

if ("name" in value) {
/// @type.node source="\"name\" in value" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in value" kind=in key_type="name" receiver={ name: string } | { id: int32 } predicate="membership({ name: string } | { id: int32 }, name)" narrowed={ name: string }
/// @type.node source=value type={ name: string } | { id: int32 }
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

    value.name satisfies string;
    /// @type.node source="value.name satisfies string" type=string
    /// @type.node source=value type={ name: string }
    /// @type.node source=value.name type=string
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.name receiver={ name: string } type=string kind=field target_receiver={ name: string } key=name target_type=string
    /// @resolution.place source=value placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=value root=value
    /// @resolution.narrowing source=value union={ name: string } | { id: int32 } arms={ name: string }
    /// @resolution.place source=value.name placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=value.name root=value keys=[name]

}
"#,
    );
}

#[test]
fn test_in_narrows_negative_branch_by_property() {
    let session = TestSession::single(
        r#"
type Named = { name: string };
type Numbered = { id: int32 };

declare const value: Named | Numbered;

if ("name" in value) {
} else {
    value.id satisfies int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Named = { name: string };
type Numbered = { id: int32 };

declare const value: { name: string } | { id: int32 };

if ("name" in value) {
} else {
    value.id satisfies int32;
}

=== dir ===
type Named = { name: string };
/// @type.symbol symbol=Named source="type Named = { name: string }" type={ name: string }
/// @definition.type symbol=Named source="type Named = { name: string }" value={ name: string }
/// @type.symbol symbol=Named.name source="name: string" type=string

type Numbered = { id: int32 };
/// @type.symbol symbol=Numbered source="type Numbered = { id: int32 }" type={ id: int32 }
/// @definition.type symbol=Numbered source="type Numbered = { id: int32 }" value={ id: int32 }
/// @type.symbol symbol=Numbered.id source="id: int32" type=int32

declare const value: Named | Numbered;
/// @type.symbol symbol=value source=value type={ name: string } | { id: int32 }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Named target=Named
/// @resolution.name source=Numbered target=Numbered

if ("name" in value) {
/// @type.node source="\"name\" in value" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in value" kind=in key_type="name" receiver={ name: string } | { id: int32 } predicate="membership({ name: string } | { id: int32 }, name)" narrowed={ name: string }
/// @type.node source=value type={ name: string } | { id: int32 }
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

} else {
    value.id satisfies int32;
    /// @type.node source="value.id satisfies int32" type=int32
    /// @type.node source=value type={ id: int32 }
    /// @type.node source=value.id type=int32
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.id receiver={ id: int32 } type=int32 kind=field target_receiver={ id: int32 } key=id target_type=int32
    /// @resolution.place source=value placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=value root=value
    /// @resolution.narrowing source=value union={ name: string } | { id: int32 } arms={ id: int32 }
    /// @resolution.place source=value.id placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=value.id root=value keys=[id]

}
"#,
    );
}

#[test]
fn test_in_rejects_primitive_receiver() {
    let session = TestSession::single(
        r#"
"x" in 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
"x" in 1;

=== dir ===
"x" in 1;
/// @type.node source="\"x\" in 1" type=boolean
/// @type.node source="\"x\"" type="x"
/// @resolution.guard source="\"x\" in 1" kind=in key_type="x" receiver=1 predicate="membership(1, x)" narrowed=never
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator 'in' is not defined for '\"x\"' and '1'"
/// @diagnostic.label line=2 column=5 span="in" line_source="\"x\" in 1;"
"#,
    );
}

#[test]
fn test_in_rejects_non_key_operands() {
    let session = TestSession::single(
        r#"
const point = { x: 1 };

true in point;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const point: { x: int64 } = { x: 1 };

true in point;

=== dir ===
const point = { x: 1 };
/// @type.symbol symbol=point source=point type={ x: int64 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source={ x: 1 } type={ x: int64 }
/// @type.node source=1 type=1

true in point;
/// @type.node source="true in point" type=boolean
/// @type.node source=true type=true
/// @resolution.guard source="true in point" kind=in key_type=true receiver={ x: int64 } predicate="membership({ x: int64 }, true)"
/// @type.node source=point type={ x: int64 }
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator 'in' is not defined for 'true' and '{ x: {integer} }'"
/// @diagnostic.label line=4 column=6 span="in" line_source="true in point;"
"#,
    );
}

#[test]
fn test_in_rejects_unknown_receiver() {
    let session = TestSession::single(
        r#"
declare const value: unknown;

"name" in value;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: unknown;

"name" in value;

=== dir ===
declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown
/// @resolution.pattern source=value kind=binding target=value

"name" in value;
/// @type.node source="\"name\" in value" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in value" kind=in key_type="name" receiver=unknown predicate="membership(unknown, name)" narrowed={ readonly name: unknown }
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator 'in' is not defined for '\"name\"' and 'unknown'"
/// @diagnostic.label line=4 column=8 span="in" line_source="\"name\" in value;"
"#,
    );
}
