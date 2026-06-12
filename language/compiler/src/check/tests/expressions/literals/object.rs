use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_literal_infers_property_types() {
    let session = TestSession::single(
        r#"
const value = { a: 1, b: "two" };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: { a: float64; b: string } = { a: 1, b: "two" };

=== checked ===
const value = { a: 1, b: "two" };
/// @type.symbol symbol=value source=value type=Managed<{ a: float64; b: string }>
/// @type.node source="{ a: 1, b: \"two\" }" type=Managed<{ a: 1; b: "two" }>
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

/// @check.stats.solve variables=0 types=9 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_object_literal_uses_shorthand_binding_types() {
    let session = TestSession::single(
        r#"
const name = "Ada";
const age = 42;
const person = { name, age };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const name: "Ada" = "Ada";
const age: 42 = 42;
const person: { name: string; age: float64 } = { name, age };

=== checked ===
const name = "Ada";
/// @type.symbol symbol=name source=name type="Ada"
/// @type.node source="\"Ada\"" type="Ada"

const age = 42;
/// @type.symbol symbol=age source=age type=42
/// @type.node source=42 type=42

const person = { name, age };
/// @type.symbol symbol=person source=person type=Managed<{ name: string; age: float64 }>
/// @type.node source="{ name, age }" type=Managed<{ name: "Ada"; age: 42 }>
/// @type.node source=name type="Ada"
/// @resolution.name source=name target=name
/// @type.node source=age type=42
/// @resolution.name source=age target=age

/// @check.stats.solve variables=0 types=11 constraints=0 obligations=0 solutions=0 bounds=0 decisions=2
"#,
    );
}

#[test]
fn test_empty_object_literal_has_empty_shape() {
    let session = TestSession::single(
        r#"
const value = {};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: {} = {};

=== checked ===
const value = {};
/// @type.symbol symbol=value source=value type=Managed<{  }>
/// @type.node source={} type=Managed<{  }>

/// @check.stats.solve variables=0 types=5 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_const_asserted_object_preserves_literal_properties() {
    let session = TestSession::single(
        r#"
const value = { a: 1, b: "two" } as const;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: { a: 1; b: "two" } = { a: 1, b: "two" } as const;

=== checked ===
const value = { a: 1, b: "two" } as const;
/// @type.symbol symbol=value source=value type=Managed<{ a: 1; b: "two" }>
/// @type.node source="{ a: 1, b: \"two\" } as const" type=Managed<{ a: 1; b: "two" }>
/// @type.node source="{ a: 1, b: \"two\" }" type=Managed<{ a: 1; b: "two" }>
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

/// @check.stats.solve variables=0 types=5 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_contextual_object_literal_rejects_property_mismatch() {
    let session = TestSession::single(
        r#"
const value: { a: number; b: string } = { a: 1, b: 2 };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: { a: number; b: string } = { a: 1, b: 2 };

=== checked ===
const value: { a: number; b: string } = { a: 1, b: 2 };
/// @type.symbol symbol=value source=value type={ a: float64; b: string }
/// @type.node source="{ a: 1, b: 2 }" type=Managed<{ a: 1; b: 2 }>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=8 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type '{ a: 1; b: 2 }' is not assignable to type '{ a: float64; b: string }'"
/// @diagnostic.label line=2 column=41 source="const value: { a: number; b: string } = { a: 1, b: 2 };"
"#,
    );
}

#[test]
fn test_contextual_object_literal_contextualizes_empty_array_field() {
    let session = TestSession::single(
        r#"
const state: { reactions: int32[] } = { reactions: [] };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const state: { reactions: int32[] } = { reactions: [] };

=== checked ===
const state: { reactions: int32[] } = { reactions: [] };
/// @type.symbol symbol=state source=state type={ reactions: Array<int32> }
/// @type.node source="{ reactions: [] }" type=Managed<{ reactions: Array<int32> }>
/// @type.node source=[] type=Array<int32>

/// @check.stats.solve variables=1 types=8 constraints=2 obligations=0 solutions=1 bounds=2 decisions=0
"#,
    );
}

#[test]
fn test_satisfies_preserves_object_literal_members() {
    let session = TestSession::single(
        r#"
type Mode = "dev" | "prod";
type Shape = { mode: Mode };

let config = { mode: "dev" } satisfies Shape;
const mode = config.mode;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
type Mode = "dev" | "prod";
type Shape = { mode: Mode };

let config: { mode: "dev" } = { mode: "dev" } satisfies Shape;
const mode: "dev" = config.mode;

=== checked ===
type Mode = "dev" | "prod";
/// @type.symbol symbol=Mode source="type Mode = \"dev\" | \"prod\"" type="dev" | "prod"
/// @definition.type symbol=Mode source="type Mode = \"dev\" | \"prod\"" value="dev" | "prod"

type Shape = { mode: Mode };
/// @type.symbol symbol=Shape source="type Shape = { mode: Mode }" type={ mode: "dev" | "prod" }
/// @definition.type symbol=Shape source="type Shape = { mode: Mode }" value={ mode: "dev" | "prod" }
/// @resolution.name source=Mode target=Mode

let config = { mode: "dev" } satisfies Shape;
/// @type.symbol symbol=config source=config type=Managed<{ mode: "dev" }>
/// @type.node source="{ mode: \"dev\" } satisfies Shape" type=Managed<{ mode: "dev" }>
/// @type.node source="{ mode: \"dev\" }" type=Managed<{ mode: "dev" }>
/// @type.node source="\"dev\"" type="dev"
/// @resolution.name source=Shape target=Shape

const mode = config.mode;
/// @type.symbol symbol=mode source=mode type="dev"
/// @type.node source=config type=Managed<{ mode: "dev" }>
/// @type.node source=config.mode type="dev"
/// @resolution.name source=config target=config
/// @resolution.member source=config.mode receiver=Managed<{ mode: "dev" }> kind=field key=mode

/// @check.stats.solve variables=2 types=13 constraints=2 obligations=0 solutions=2 bounds=2 decisions=4
"#,
    );
}

#[test]
fn test_object_spread_adds_fields() {
    let session = TestSession::single(
        r#"
const base = { a: 1, b: "two" };
const value = { ...base, c: true };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const base: { a: float64; b: string } = { a: 1, b: "two" };
const value: { a: float64; b: string; c: true } = { ...base, c: true };

=== checked ===
const base = { a: 1, b: "two" };
/// @type.symbol symbol=base source=base type=Managed<{ a: float64; b: string }>
/// @type.node source="{ a: 1, b: \"two\" }" type=Managed<{ a: 1; b: "two" }>
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

const value = { ...base, c: true };
/// @type.symbol symbol=value source=value type=Managed<{ a: float64; b: string; c: true }>
/// @type.node source="{ ...base, c: true }" type=Managed<{ a: float64; b: string; c: true }>
/// @type.node source=base type=Managed<{ a: float64; b: string }>
/// @resolution.name source=base target=base
/// @type.node source=true type=true

/// @check.stats.solve variables=2 types=15 constraints=1 obligations=0 solutions=2 bounds=2 decisions=1
"#,
    );
}

#[test]
fn test_object_spread_overrides_fields() {
    let session = TestSession::single(
        r#"
const base = { a: 1, b: 2 };
const value = { ...base, b: "two" };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const base: { a: float64; b: float64 } = { a: 1, b: 2 };
const value: { a: float64; b: "two" } = { ...base, b: "two" };

=== checked ===
const base = { a: 1, b: 2 };
/// @type.symbol symbol=base source=base type=Managed<{ a: float64; b: float64 }>
/// @type.node source="{ a: 1, b: 2 }" type=Managed<{ a: 1; b: 2 }>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const value = { ...base, b: "two" };
/// @type.symbol symbol=value source=value type=Managed<{ a: float64; b: "two" }>
/// @type.node source="{ ...base, b: \"two\" }" type=Managed<{ a: float64; b: "two" }>
/// @type.node source=base type=Managed<{ a: float64; b: float64 }>
/// @resolution.name source=base target=base
/// @type.node source="\"two\"" type="two"

/// @check.stats.solve variables=2 types=15 constraints=1 obligations=0 solutions=2 bounds=2 decisions=1
"#,
    );
}

#[test]
fn test_struct_update_spread_preserves_nominal_type() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
const moved = Point { ...point, x: 3 };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2 };
const moved: Point = Point { ...point, x: 3 };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Point
/// @type.node source="Point { x: 1, y: 2 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const moved = Point { ...point, x: 3 };
/// @type.symbol symbol=moved source=moved type=Point
/// @type.node source="Point { ...point, x: 3 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @type.node source=3 type=3

/// @check.stats.solve variables=3 types=15 constraints=5 obligations=1 solutions=3 bounds=4 decisions=3
"#,
    );
}

#[test]
fn test_object_spread_from_struct_erases_nominal_type() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
const object = { ...point, label: "origin" };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2 };
const object: { x: int32; y: int32; label: "origin" } = { ...point, label: "origin" };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Point
/// @type.node source="Point { x: 1, y: 2 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const object = { ...point, label: "origin" };
/// @type.symbol symbol=object source=object type=Managed<{ x: int32; y: int32; label: "origin" }>
/// @type.node source="{ ...point, label: \"origin\" }" type=Managed<{ x: int32; y: int32; label: "origin" }>
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @type.node source="\"origin\"" type="origin"

/// @check.stats.solve variables=4 types=17 constraints=5 obligations=1 solutions=4 bounds=5 decisions=2
"#,
    );
}

#[test]
fn test_struct_spread_from_object_satisfies_nominal_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const base: { x: int32; y: int32 } = { x: 1, y: 2 };
const point: Point = _ { ...base };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const base: { x: int32; y: int32 } = { x: 1, y: 2 };
const point: Point = Point { ...base };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const base: { x: int32; y: int32 } = { x: 1, y: 2 };
/// @type.symbol symbol=base source=base type={ x: int32; y: int32 }
/// @type.node source="{ x: 1, y: 2 }" type=Managed<{ x: 1; y: 2 }>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const point: Point = _ { ...base };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.name source=Point target=Point
/// @type.node source="_ { ...base }" type=Point
/// @type.node source=base type={ x: int32; y: int32 }
/// @resolution.name source=base target=base

/// @check.stats.solve variables=2 types=18 constraints=4 obligations=1 solutions=2 bounds=3 decisions=2
"#,
    );
}

#[test]
fn test_object_spread_rejects_class_instance() {
    let session = TestSession::single(
        r#"
class User {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}

const user = new User("Ada");
const object = { ...user };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::none().with_check_stats(),
        r#"
=== annotated ===
class User {
    name: string;

    constructor(name: string): User {
        this.name = name;
    }
}

const user: User = new User("Ada");
const object: { name: string } = { ...user };

=== checked ===
class User {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}

const user = new User("Ada");
const object = { ...user };

/// @check.stats.solve variables=6 types=18 constraints=6 obligations=3 solutions=6 bounds=7 decisions=6
"#,
        r#"
/// @diagnostic.error code=EC500 message="type 'User' has no concrete layout"
/// @diagnostic.label line=2 column=1 source="class User {"
"#,
    );
}
