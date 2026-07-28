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
/// @type.symbol symbol=value source=value type={ a: float64; b: string }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source={ a: 1, b: "two" } type={ a: float64; b: string }
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

/// @check.stats.solve variables=1 types=8 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
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
/// @resolution.pattern source=name kind=binding target=name
/// @type.node source="\"Ada\"" type="Ada"

const age = 42;
/// @type.symbol symbol=age source=age type=42
/// @resolution.pattern source=age kind=binding target=age
/// @type.node source=42 type=42

const person = { name, age };
/// @type.symbol symbol=person source=person type={ name: string; age: float64 }
/// @resolution.pattern source=person kind=binding target=person
/// @type.node source={ name, age } type={ name: string; age: float64 }
/// @type.node source=name type="Ada"
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=name root=name
/// @type.node source=age type=42
/// @resolution.name source=age target=age
/// @resolution.place source=age placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=age root=age

/// @check.stats.solve variables=3 types=13 constraints=0 obligations=3 solutions=3 bounds=0 decisions=5
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
/// @type.symbol symbol=value source=value type={}
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source={} type={}

/// @check.stats.solve variables=1 types=3 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
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
const value: { readonly a: 1; readonly b: "two" } = { a: 1, b: "two" } as const;

=== checked ===
const value = { a: 1, b: "two" } as const;
/// @type.symbol symbol=value source=value type={ readonly a: 1; readonly b: "two" }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="{ a: 1, b: \"two\" } as const" type={ readonly a: 1; readonly b: "two" }
/// @type.node source={ a: 1, b: "two" } type={ readonly a: 1; readonly b: "two" }
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

/// @check.stats.solve variables=1 types=5 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
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
const value: { a: float64; b: string } = { a: 1, b: 2 };

=== checked ===
const value: { a: number; b: string } = { a: 1, b: 2 };
/// @type.symbol symbol=value source=value type={ a: float64; b: string }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source={ a: 1, b: 2 } type={ a: float64; b: string }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=1 types=7 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '2' is not assignable to type 'string'"
/// @diagnostic.label line=2 column=52 span="2" line_source="const value: { a: number; b: string } = { a: 1, b: 2 };"
/// @diagnostic.related line=2 column=14 span="{ a: number; b: string }" line_source="const value: { a: number; b: string } = { a: 1, b: 2 };" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'b'"
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
/// @resolution.pattern source=state kind=binding target=state
/// @type.node source={ reactions: [] } type={ reactions: Array<int32> }
/// @type.node source=[] type=Array<int32>

/// @check.stats.solve variables=1 types=5 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_contextual_object_literal_checks_index_signature_fields() {
    let session = TestSession::single(
        r#"
type Counts = { [key: string]: int32 };

const counts: Counts = { apples: 1, oranges: 2 };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            .with_check_stats(),
        r#"
=== annotated ===
type Counts = { [key: string]: int32 };

const counts: Counts = { apples: 1, oranges: 2 };

=== checked ===
type Counts = { [key: string]: int32 };
/// @type.symbol symbol=Counts source="type Counts = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Counts source="type Counts = { [key: string]: int32 }" value={ [key: string]: int32 }

const counts: Counts = { apples: 1, oranges: 2 };
/// @type.symbol symbol=counts source=counts type=Counts reduced={ [key: string]: int32 }
/// @resolution.pattern source=counts kind=binding target=counts
/// @resolution.name source=Counts target=Counts
/// @type.node source={ apples: 1, oranges: 2 } type={ [key: string]: int32 }
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: int32 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: int32 }] origin=implicit

/// @check.stats.solve variables=1 types=10 constraints=0 obligations=1 solutions=1 bounds=0 decisions=2
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
/// @type.symbol symbol=Shape source="type Shape = { mode: Mode }" type={ mode: Mode }
/// @definition.type symbol=Shape source="type Shape = { mode: Mode }" value={ mode: Mode }
/// @resolution.name source=Mode target=Mode

let config = { mode: "dev" } satisfies Shape;
/// @type.symbol symbol=config source=config type={ mode: "dev" }
/// @resolution.pattern source=config kind=binding target=config
/// @type.node source="{ mode: \"dev\" } satisfies Shape" type={ mode: "dev" }
/// @type.node source={ mode: "dev" } type={ mode: "dev" }
/// @type.node source="\"dev\"" type="dev"
/// @resolution.name source=Shape target=Shape

const mode = config.mode;
/// @type.symbol symbol=mode source=mode type="dev"
/// @resolution.pattern source=mode kind=binding target=mode
/// @type.node source=config type={ mode: "dev" }
/// @type.node source=config.mode type="dev"
/// @resolution.name source=config target=config
/// @resolution.member source=config.mode receiver={ mode: "dev" } type="dev" kind=field target_receiver={ mode: "dev" } key=mode target_type="dev"
/// @resolution.place source=config placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=config root=config
/// @resolution.access source=config.mode root=config keys=[mode]

/// @check.stats.solve variables=2 types=14 constraints=0 obligations=2 solutions=2 bounds=0 decisions=6
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
const value: { a: float64; b: string; c: boolean } = { ...base, c: true };

=== checked ===
const base = { a: 1, b: "two" };
/// @type.symbol symbol=base source=base type={ a: float64; b: string }
/// @resolution.pattern source=base kind=binding target=base
/// @type.node source={ a: 1, b: "two" } type={ a: float64; b: string }
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

const value = { ...base, c: true };
/// @type.symbol symbol=value source=value type={ a: float64; b: string; c: boolean }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source={ ...base, c: true } type={ a: float64; b: string; c: boolean }
/// @type.node source=base type={ a: float64; b: string }
/// @resolution.name source=base target=base
/// @resolution.access source=base root=base
/// @type.node source=true type=true

/// @check.stats.solve variables=2 types=13 constraints=0 obligations=2 solutions=2 bounds=0 decisions=3
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
const value: { a: float64; b: string } = { ...base, b: "two" };

=== checked ===
const base = { a: 1, b: 2 };
/// @type.symbol symbol=base source=base type={ a: float64; b: float64 }
/// @resolution.pattern source=base kind=binding target=base
/// @type.node source={ a: 1, b: 2 } type={ a: float64; b: float64 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const value = { ...base, b: "two" };
/// @type.symbol symbol=value source=value type={ a: float64; b: string }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source={ ...base, b: "two" } type={ a: float64; b: string }
/// @type.node source=base type={ a: float64; b: float64 }
/// @resolution.name source=base target=base
/// @resolution.access source=base root=base
/// @type.node source="\"two\"" type="two"

/// @check.stats.solve variables=2 types=12 constraints=0 obligations=2 solutions=2 bounds=0 decisions=3
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
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1, y: 2 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const moved = Point { ...point, x: 3 };
/// @type.symbol symbol=moved source=moved type=Point
/// @resolution.pattern source=moved kind=binding target=moved
/// @type.node source="Point { ...point, x: 3 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
/// @type.node source=3 type=3

/// @check.stats.solve variables=2 types=14 constraints=0 obligations=4 solutions=2 bounds=0 decisions=5
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
const object: { x: int32; y: int32; label: string } = { ...point, label: "origin" };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1, y: 2 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const object = { ...point, label: "origin" };
/// @type.symbol symbol=object source=object type={ x: int32; y: int32; label: string }
/// @resolution.pattern source=object kind=binding target=object
/// @type.node source={ ...point, label: "origin" } type={ x: int32; y: int32; label: string }
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point
/// @type.node source="\"origin\"" type="origin"

/// @check.stats.solve variables=2 types=13 constraints=0 obligations=4 solutions=2 bounds=0 decisions=4
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
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const base: { x: int32; y: int32 } = { x: 1, y: 2 };
/// @type.symbol symbol=base source=base type={ x: int32; y: int32 }
/// @resolution.pattern source=base kind=binding target=base
/// @type.node source={ x: 1, y: 2 } type={ x: int32; y: int32 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const point: Point = _ { ...base };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
/// @type.node source="_ { ...base }" type=Point
/// @type.node source=base type={ x: int32; y: int32 }
/// @resolution.name source=base target=base
/// @resolution.place source=base placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=base root=base

/// @check.stats.solve variables=2 types=12 constraints=0 obligations=4 solutions=2 bounds=0 decisions=4
"#,
    );
}

#[test]
fn test_object_spread_from_class_erases_nominal_type() {
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
class User {
    name: string;

    constructor(name: string): this {
        this.name = name;
    }
}

const user: User = new User("Ada");
const object: { name: string } = { ...user };

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(string) => this

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=User.constructor type=(string) => this
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string

        this.name = name;
        /// @type.node source="this.name = name" type=string
        /// @type.node source=this type=User
        /// @type.node source=this.name type=string
        /// @resolution.receiver source=this kind=this declaration=User type=User
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.assignment source=this.name write="receiver=User, target=field(receiver=User, target=User.name, type=string), type=string" type=string
        /// @type.node source=name type=string
        /// @resolution.name source=name target=User.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=User.constructor.name

    }
}

const user = new User("Ada");
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @type.node source="new User(\"Ada\")" type=User
/// @resolution.construct source="new User(\"Ada\")" parameters=(string) arguments=(provided("Ada") as string) return=User kind=class target=User constructor=User.constructor
/// @resolution.name source=User target=User
/// @type.node source="\"Ada\"" type="Ada"

const object = { ...user };
/// @type.symbol symbol=object source=object type={ name: string }
/// @resolution.pattern source=object kind=binding target=object
/// @type.node source={ ...user } type={ name: string }
/// @type.node source=user type=User
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user

/// @check.stats.solve variables=2 types=15 constraints=0 obligations=6 solutions=2 bounds=0 decisions=9
"#,
    );
}
