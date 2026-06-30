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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const point: { x: float64; y: float64 } = { x: 1, y: 2 };

const hasX: boolean = "x" in point;
hasX satisfies boolean;

=== checked ===
const point = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: float64; y: float64 }
/// @type.node source={ x: 1, y: 2 } type={ x: 1; y: 2 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const hasX = "x" in point;
/// @type.symbol symbol=hasX source=hasX type=boolean
/// @type.node source="\"x\" in point" type=boolean
/// @type.node source="\"x\"" type="x"
/// @resolution.guard source="\"x\" in point" kind=in key_type="x" receiver={ x: float64; y: float64 } predicate="has({ x: float64; y: float64 }, x)" narrowed={ x: float64; y: float64 }
/// @type.node source=point type={ x: float64; y: float64 }
/// @resolution.name source=point target=point

hasX satisfies boolean;
/// @type.node source="hasX satisfies boolean" type=boolean
/// @type.node source=hasX type=boolean
/// @resolution.name source=hasX target=hasX
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const point: { x: float64; y: float64 } = { x: 1, y: 2 };

const hasName: boolean = "name" in point;
hasName satisfies boolean;

=== checked ===
const point = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: float64; y: float64 }
/// @type.node source={ x: 1, y: 2 } type={ x: 1; y: 2 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const hasName = "name" in point;
/// @type.symbol symbol=hasName source=hasName type=boolean
/// @type.node source="\"name\" in point" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in point" kind=in key_type="name" receiver={ x: float64; y: float64 } predicate="has({ x: float64; y: float64 }, name)" narrowed=never
/// @type.node source=point type={ x: float64; y: float64 }
/// @resolution.name source=point target=point

hasName satisfies boolean;
/// @type.node source="hasName satisfies boolean" type=boolean
/// @type.node source=hasName type=boolean
/// @resolution.name source=hasName target=hasName
"#,
    );
}

#[test]
fn test_in_dispatches_to_has_for_nominal_receiver() {
    let session = TestSession::single(
        r#"
class Bag implements Has<string> {
    has(key: &readonly string): boolean {
        return true;
    }
}

declare const bag: Bag;

const found = "name" in bag;
found satisfies boolean;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Bag implements Has<string> {
    has(key: Borrowed<string, L0, "readonly">): boolean {
        return true;
    }
}

declare const bag: Bag;

const found: boolean = "name" in bag;
found satisfies boolean;

=== checked ===
class Bag implements Has<string> {
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag
/// @definition.implements symbol=Bag source=Has<string> target=ops.subscript.Has arguments=(string)
/// @definition.method symbol=Bag.has slot=has type=<comptime has.L0: Lifetime>(this: Bag, Borrowed<string, has.L0, "readonly">) => boolean
/// @resolution.name source=Has target=ops.subscript.Has

    has(key: &readonly string): boolean {
    /// @generic.template symbol=Bag.has parameters=(comptime L0: Lifetime origin=induced.form)
    /// @type.symbol symbol=Bag.has type=<comptime has.L0: Lifetime>(this: Bag, Borrowed<string, has.L0, "readonly">) => boolean
    /// @type.symbol symbol=key source="key: &readonly string" type=Borrowed<string, has.L0, "readonly">

        return true;
        /// @type.node source=true type=true

    }
}

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag
/// @resolution.name source=Bag target=Bag

const found = "name" in bag;
/// @type.symbol symbol=found source=found type=boolean
/// @type.node source="\"name\" in bag" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in bag" kind=in key_type="name" receiver=Bag predicate=call(Bag.has)
/// @generic.instance source="\"name\" in bag" id="Bag.has<\"frame\">"
/// @type.node source=bag type=Bag
/// @resolution.name source=bag target=bag

found satisfies boolean;
/// @type.node source="found satisfies boolean" type=boolean
/// @type.node source=found type=boolean
/// @resolution.name source=found target=found

/// @generic.instance id="Bag.has<\"frame\">" template=Bag.has arguments=("frame")
"#,
    );
}

#[test]
fn test_in_rejects_nominal_receiver_without_has() {
    let session = TestSession::single(
        r#"
declare class User {
    name: string;
}

declare const user: User;

"name" in user;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class User {
    name: string;
}

declare const user: User;

"name" in user;

=== checked ===
declare class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User

"name" in user;
/// @type.node source="\"name\"" type="name"
/// @type.node source=user type=User
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error code=EC306 message="operator 'in' is not defined for '\"name\"' and 'User'"
/// @diagnostic.label line=8 column=8 span="in" line_source="\"name\" in user;"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Named = { name: string };
type Numbered = { id: int32 };

declare const value: Named | Numbered;

if ("name" in value) {
    value.name satisfies string;
}

=== checked ===
type Named = { name: string };
/// @type.symbol symbol=Named source="type Named = { name: string }" type={ name: string }
/// @definition.type symbol=Named source="type Named = { name: string }" value={ name: string }

type Numbered = { id: int32 };
/// @type.symbol symbol=Numbered source="type Numbered = { id: int32 }" type={ id: int32 }
/// @definition.type symbol=Numbered source="type Numbered = { id: int32 }" value={ id: int32 }

declare const value: Named | Numbered;
/// @type.symbol symbol=value source=value type=Named | Numbered
/// @resolution.name source=Named target=Named
/// @resolution.name source=Numbered target=Numbered

if ("name" in value) {
/// @type.node source="\"name\" in value" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in value" kind=in key_type="name" receiver=Named | Numbered predicate="has(Named | Numbered, name)" narrowed={ name: string }
/// @type.node source=value type={ name: string } | { id: int32 }
/// @resolution.name source=value target=value

    value.name satisfies string;
    /// @type.node source="value.name satisfies string" type=string
    /// @type.node source=value type={ name: string }
    /// @type.node source=value.name type=string
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.name receiver={ name: string } kind=field key=name

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
"x" in 1;

=== checked ===
"x" in 1;
/// @type.node source="\"x\" in 1" type=boolean
/// @type.node source="\"x\"" type="x"
/// @resolution.guard source="\"x\" in 1" kind=in key_type="x" receiver=1 predicate="has(1, x)" narrowed=never
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error code=EC306 message="operator 'in' is not defined for '\"x\"' and '1'"
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const point: { x: float64 } = { x: 1 };

true in point;

=== checked ===
const point = { x: 1 };
/// @type.symbol symbol=point source=point type={ x: float64 }
/// @type.node source={ x: 1 } type={ x: 1 }
/// @type.node source=1 type=1

true in point;
/// @type.node source="true in point" type=boolean
/// @type.node source=true type=true
/// @resolution.guard source="true in point" kind=in key_type=true receiver={ x: float64 } predicate="has({ x: float64 }, true)"
/// @type.node source=point type={ x: float64 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC306 message="operator 'in' is not defined for 'true' and '{ x: float64 }'"
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: unknown;

"name" in value;

=== checked ===
declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown

"name" in value;
/// @type.node source="\"name\" in value" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.guard source="\"name\" in value" kind=in key_type="name" receiver=unknown predicate="has(unknown, name)" narrowed={ name: unknown }
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
"#,
        r#"
/// @diagnostic.error code=EC306 message="operator 'in' is not defined for '\"name\"' and 'unknown'"
/// @diagnostic.label line=4 column=8 span="in" line_source="\"name\" in value;"
"#,
    );
}
