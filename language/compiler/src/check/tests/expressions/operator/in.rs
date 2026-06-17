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
/// @type.symbol symbol=point source=point type=Managed<{ x: float64; y: float64 }>
/// @type.node source="{ x: 1, y: 2 }" type=Managed<{ x: float64; y: float64 }>
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64

const hasX = "x" in point;
/// @type.symbol symbol=hasX source=hasX type=boolean
/// @type.node source="\"x\" in point" type=boolean
/// @type.node source="\"x\"" type="x"
/// @resolution.call source="\"x\" in point" parameters=() return=boolean kind=builtin builtin=binary.in
/// @type.node source=point type=Managed<{ x: float64; y: float64 }>
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
/// @type.symbol symbol=point source=point type=Managed<{ x: float64; y: float64 }>
/// @type.node source="{ x: 1, y: 2 }" type=Managed<{ x: float64; y: float64 }>
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64

const hasName = "name" in point;
/// @type.symbol symbol=hasName source=hasName type=boolean
/// @type.node source="\"name\" in point" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.call source="\"name\" in point" parameters=() return=boolean kind=builtin builtin=binary.in
/// @type.node source=point type=Managed<{ x: float64; y: float64 }>
/// @resolution.name source=point target=point

hasName satisfies boolean;
/// @type.node source="hasName satisfies boolean" type=boolean
/// @type.node source=hasName type=boolean
/// @resolution.name source=hasName target=hasName
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
/// @type.symbol symbol=value source=value type={ name: string } | { id: int32 }
/// @resolution.name source=Named target=Named
/// @resolution.name source=Numbered target=Numbered

if ("name" in value) {
/// @type.node type=void | void
/// @type.node source="\"name\" in value" type=boolean
/// @type.node source="\"name\"" type="name"
/// @resolution.call source="\"name\" in value" parameters=() return=boolean kind=builtin builtin=binary.in
/// @type.node source=value type={ name: string } | { id: int32 }
/// @resolution.name source=value target=value

    value.name satisfies string;
    /// @type.node source="value.name satisfies string" type=string
    /// @type.node source=value type={ name: string } | { id: int32 } extends { name: unknown } ? { name: string } | { id: int32 } : never
    /// @type.node source=value.name type=string
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.name receiver={ name: string } | { id: int32 } extends { name: unknown } ? { name: string } | { id: int32 } : never kind=field key=name

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
/// @type.node source="\"x\" in 1" type=<error>
/// @type.node source="\"x\"" type="x"
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error code=EC306 message="operator 'in' is not defined for '\"x\"' and '1'"
/// @diagnostic.label line=2 column=1 source="\"x\" in 1;"
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
/// @type.symbol symbol=point source=point type=Managed<{ x: float64 }>
/// @type.node source="{ x: 1 }" type=Managed<{ x: float64 }>
/// @type.node source=1 type=float64

true in point;
/// @type.node source="true in point" type=<error>
/// @type.node source=true type=true
/// @type.node source=point type=Managed<{ x: float64 }>
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC306 message="operator 'in' is not defined for 'true' and '{ x: float64 }'"
/// @diagnostic.label line=4 column=1 source="true in point;"
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
/// @type.node source="\"name\" in value" type=<error>
/// @type.node source="\"name\"" type="name"
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
"#,
        r#"
/// @diagnostic.error code=EC306 message="operator 'in' is not defined for '\"name\"' and 'unknown'"
/// @diagnostic.label line=4 column=1 source="\"name\" in value;"
"#,
    );
}
