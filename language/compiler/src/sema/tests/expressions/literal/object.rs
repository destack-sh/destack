use crate::tests::{DirRows, TestSession};

#[test]
fn test_infer_property_types_from_an_object_literal() {
    let session = TestSession::single(
        r#"
const value = { a: 1, b: "two" };
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: { a: int64; b: string } = { a: 1, b: "two" };

=== dir ===
const value = { a: 1, b: "two" };
/// @type.symbol symbol=value source=value type={ a: int64; b: string }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source={ a: 1, b: "two" } type={ a: int64; b: string }
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
"#,
    );
}

#[test]
fn test_infer_property_types_from_shorthand_object_bindings() {
    let session = TestSession::single(
        r#"
const name = "Ada";
const age = 42;
const person = { name, age };
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const name: "Ada" = "Ada";
const age: 42 = 42;
const person: { name: string; age: int64 } = { name, age };

=== dir ===
const name = "Ada";
/// @type.symbol symbol=name source=name type="Ada"
/// @resolution.pattern source=name kind=binding target=name
/// @type.node source="\"Ada\"" type="Ada"

const age = 42;
/// @type.symbol symbol=age source=age type=42
/// @resolution.pattern source=age kind=binding target=age
/// @type.node source=42 type=42

const person = { name, age };
/// @type.symbol symbol=person source=person type={ name: string; age: int64 }
/// @resolution.pattern source=person kind=binding target=person
/// @type.node source={ name, age } type={ name: string; age: int64 }
/// @type.node source=name type="Ada"
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="immutable"
/// @resolution.access source=name root=name
/// @type.node source=age type=42
/// @resolution.name source=age target=age
/// @resolution.place source=age placement="local" lifetime="static" access="immutable"
/// @resolution.access source=age root=age
"#,
    );
}

#[test]
fn test_infer_an_empty_shape_for_an_empty_object_literal() {
    let session = TestSession::single(
        r#"
const value = {};
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: {} = {};

=== dir ===
const value = {};
/// @type.symbol symbol=value source=value type={}
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source={} type={}
"#,
    );
}

#[test]
fn test_preserve_literal_properties_in_a_const_asserted_object() {
    let session = TestSession::single(
        r#"
const value = { a: 1, b: "two" } as const;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: { readonly a: 1; readonly b: "two" } = { a: 1, b: "two" } as const;

=== dir ===
const value = { a: 1, b: "two" } as const;
/// @type.symbol symbol=value source=value type={ readonly a: 1; readonly b: "two" }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="{ a: 1, b: \"two\" } as const" type={ readonly a: 1; readonly b: "two" }
/// @type.node source={ a: 1, b: "two" } type={ readonly a: 1; readonly b: "two" }
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
"#,
    );
}

#[test]
fn test_reject_a_property_mismatch_in_a_contextual_object_literal() {
    let session = TestSession::single(
        r#"
const value: { a: number; b: string } = { a: 1, b: 2 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: { a: float64; b: string } = { a: 1, b: 2 };

=== dir ===
const value: { a: number; b: string } = { a: 1, b: 2 };
/// @type.symbol symbol=value source=value type={ a: float64; b: string }
/// @resolution.pattern source=value kind=binding target=value
/// @type.symbol symbol=a source="a: number" type=float64
/// @type.symbol symbol=b source="b: string" type=string
/// @type.node source={ a: 1, b: 2 } type={ a: float64; b: string }
/// @type.node source=1 type=1
/// @type.node source=2 type=2
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
fn test_contextualize_an_empty_array_field_in_an_object_literal() {
    let session = TestSession::single(
        r#"
const state: { reactions: int32[] } = { reactions: [] };
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const state: { reactions: int32[] } = { reactions: [] };

=== dir ===
const state: { reactions: int32[] } = { reactions: [] };
/// @type.symbol symbol=state source=state type={ reactions: int32[] }
/// @resolution.pattern source=state kind=binding target=state
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=reactions source="reactions: int32[]" type=int32[]
/// @type.node source={ reactions: [] } type={ reactions: int32[] }
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(^Slice<int32>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
"#,
    );
}

#[test]
fn test_check_index_signature_fields_in_a_contextual_object_literal() {
    let session = TestSession::single(
        r#"
type Counts = { [key: string]: int32 };

const counts: Counts = { apples: 1, oranges: 2 };
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
type Counts = { [key: string]: int32 };

const counts: Counts = { apples: 1, oranges: 2 } as Counts;

=== dir ===
type Counts = { [key: string]: int32 };
/// @type.symbol symbol=Counts source="type Counts = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Counts source="type Counts = { [key: string]: int32 }" value={ [key: string]: int32 }

const counts: Counts = { apples: 1, oranges: 2 };
/// @type.symbol symbol=counts source=counts type=Counts
/// @resolution.pattern source=counts kind=binding target=counts
/// @resolution.name source=Counts target=Counts
/// @type.node source={ apples: 1, oranges: 2 } type={ apples: int32; oranges: int32 }
/// @coercion.node source={ apples: 1, oranges: 2 } from={ apples: int32; oranges: int32 } adjustments=[{ kind: erase, target: Counts }] origin=implicit
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int32 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: materialize, target: int32 }] origin=implicit
"#,
    );
}

#[test]
fn test_preserve_object_literal_members_through_a_satisfies_expression() {
    let session = TestSession::single(
        r#"
type Mode = "dev" | "prod";
type Shape = { mode: Mode };

let config = { mode: "dev" } satisfies Shape;
const mode = config.mode;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Mode = "dev" | "prod";
type Shape = { mode: Mode };

let config: { mode: "dev" } = { mode: "dev" } satisfies Shape;
const mode: "dev" = config.mode;

=== dir ===
type Mode = "dev" | "prod";
/// @type.symbol symbol=Mode source="type Mode = \"dev\" | \"prod\"" type="dev" | "prod"
/// @definition.type symbol=Mode source="type Mode = \"dev\" | \"prod\"" value="dev" | "prod"

type Shape = { mode: Mode };
/// @type.symbol symbol=Shape source="type Shape = { mode: Mode }" type={ mode: Mode }
/// @definition.type symbol=Shape source="type Shape = { mode: Mode }" value={ mode: Mode }
/// @type.symbol symbol=Shape.mode source="mode: Mode" type=Mode
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
"#,
    );
}

#[test]
fn test_add_fields_through_an_object_spread() {
    let session = TestSession::single(
        r#"
const base = { a: 1, b: "two" };
const value = { ...base, c: true };
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const base: { a: int64; b: string } = { a: 1, b: "two" };
const value: { a: int64; b: string; c: boolean } = { ...base, c: true };

=== dir ===
const base = { a: 1, b: "two" };
/// @type.symbol symbol=base source=base type={ a: int64; b: string }
/// @resolution.pattern source=base kind=binding target=base
/// @type.node source={ a: 1, b: "two" } type={ a: int64; b: string }
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

const value = { ...base, c: true };
/// @type.symbol symbol=value source=value type={ a: int64; b: string; c: boolean }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source={ ...base, c: true } type={ a: int64; b: string; c: boolean }
/// @type.node source=base type={ a: int64; b: string }
/// @resolution.name source=base target=base
/// @resolution.access source=base root=base
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_override_fields_through_an_object_spread() {
    let session = TestSession::single(
        r#"
const base = { a: 1, b: 2 };
const value = { ...base, b: "two" };
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const base: { a: int64; b: int64 } = { a: 1, b: 2 };
const value: { a: int64; b: string } = { ...base, b: "two" };

=== dir ===
const base = { a: 1, b: 2 };
/// @type.symbol symbol=base source=base type={ a: int64; b: int64 }
/// @resolution.pattern source=base kind=binding target=base
/// @type.node source={ a: 1, b: 2 } type={ a: int64; b: int64 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const value = { ...base, b: "two" };
/// @type.symbol symbol=value source=value type={ a: int64; b: string }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source={ ...base, b: "two" } type={ a: int64; b: string }
/// @type.node source=base type={ a: int64; b: int64 }
/// @resolution.name source=base target=base
/// @resolution.access source=base root=base
/// @type.node source="\"two\"" type="two"
"#,
    );
}

#[test]
fn test_preserve_nominal_type_through_a_struct_update_spread() {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2 };
const moved: Point = Point { ...point, x: 3 };

=== dir ===
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
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
/// @type.node source=3 type=3
"#,
    );
}

#[test]
fn test_erase_nominal_type_through_an_object_spread_from_a_struct() {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2 };
const object: { x: int32; y: int32; label: string } = { ...point, label: "origin" };

=== dir ===
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
"#,
    );
}

#[test]
fn test_satisfy_nominal_fields_through_a_struct_spread_from_an_object() {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const base: { x: int32; y: int32 } = { x: 1, y: 2 };
const point: Point = Point { ...base };

=== dir ===
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
/// @type.symbol symbol=x source="x: int32" type=int32
/// @type.symbol symbol=y source="y: int32" type=int32
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
/// @resolution.place source=base placement="local" lifetime="static" access="immutable"
/// @resolution.access source=base root=base
"#,
    );
}

#[test]
fn test_erase_nominal_type_through_an_object_spread_from_a_class() {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}

const user: User = new User("Ada");
const object: { name: string } = { ...user };

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(this: &'managed User, string) => User

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=User.constructor type=(this: &'managed User, string) => User
    /// @type.symbol symbol=User.constructor.this type=&'managed User
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string

        this.name = name;
        /// @type.node source="this.name = name" type=string
        /// @type.node source=this type=&'managed User
        /// @type.node source=this.name type=string
        /// @resolution.receiver source=this kind=this declaration=User type=&'managed User
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed User, target=field(receiver=&'managed User, target=User.name, type=string), type=string" type=string
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
/// @type.node source=User type=typeof User
/// @resolution.name source=User target=User
/// @type.node source="\"Ada\"" type="Ada"

const object = { ...user };
/// @type.symbol symbol=object source=object type={ name: string }
/// @resolution.pattern source=object kind=binding target=object
/// @type.node source={ ...user } type={ name: string }
/// @type.node source=user type=User
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user
"#,
    );
}

/// Reject an object literal writing one property twice.
#[test]
fn test_reject_duplicate_object_property() {
    let session = TestSession::single(
        r#"
const value = { name: "Ada", name: "Grace" };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const value: { name: string } = { name: "Ada", name: "Grace" };

=== dir ===
const value = { name: "Ada", name: "Grace" };
/// @type.symbol symbol=value source=value type={ name: string }
/// @resolution.pattern source=value kind=binding target=value
"#,
        r#"
/// @diagnostic.error id=duplicate-member message="member 'name' is already declared"
/// @diagnostic.label line=2 column=30 span="name" line_source="const value = { name: \"Ada\", name: \"Grace\" };"
"#,
    );
}

/// Infer a getter as a readable object property.
#[test]
fn test_infer_a_read_property_from_an_object_getter() {
    let session = TestSession::single(
        r#"
const store = {
    get value(): string { return "ready"; },
};
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const store: { readonly value: string } = {
    get value(): string {
        return "ready";
    },
};

=== dir ===
const store = {
/// @type.symbol symbol=store source=store type={ readonly value: string }
/// @resolution.pattern source=store kind=binding target=store
/// @type.node type={ readonly value: string }

    get value(): string { return "ready"; },
    /// @type.symbol symbol=symbol1 source="get value(): string { return \"ready\"; }" type=() => string
    /// @type.node source="\"ready\"" type="ready"

};
"#,
    );
}

/// Infer a setter as a writable object property.
#[test]
fn test_infer_a_write_property_from_an_object_setter() {
    let session = TestSession::single(
        r#"
const store = {
    set value(next: string): void {},
};
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const store: { value: string } = {
    set value(next: string): void {},
};

=== dir ===
const store = {
/// @type.symbol symbol=store source=store type={ set value(value: string) }
/// @resolution.pattern source=store kind=binding target=store
/// @type.node type={ set value(value: string) }

    set value(next: string): void {},
    /// @type.symbol symbol=symbol1 source="set value(next: string): void {}" type=(string) => void

};
"#,
    );
}

/// Merge one getter and setter into one readable and writable object property.
#[test]
fn test_merge_a_getter_and_setter_into_one_property() {
    let session = TestSession::single(
        r#"
const store = {
    get value(): string { return "ready"; },
    set value(next: string | int32): void {},
};
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const store: { value: string | int32 } = {
    get value(): string {
        return "ready";
    },
    set value(next: string | int32): void {},
};

=== dir ===
const store = {
/// @type.symbol symbol=store source=store type={ get value(): string; set value(value: string | int32) }
/// @resolution.pattern source=store kind=binding target=store
/// @type.node type={ get value(): string; set value(value: string | int32) }

    get value(): string { return "ready"; },
    /// @type.symbol symbol=symbol1 source="get value(): string { return \"ready\"; }" type=() => string
    /// @type.node source="\"ready\"" type="ready"

    set value(next: string | int32): void {},
    /// @type.symbol symbol=symbol2 source="set value(next: string | int32): void {}" type=(string | int32) => void

};
"#,
    );
}

/// Accept an object accessor pair under a structural property expectation.
#[test]
fn test_accept_an_object_accessor_pair_under_a_structural_property_expectation() {
    let session = TestSession::single(
        r#"
interface Store {
    get value(): string;
    set value(next: string | int32);
}
const store: Store = {
    get value(): string { return "ready"; },
    set value(next: string | int32): void {},
};
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Store {
    get value(): string;
    set value(next: string | int32);
}
const store: Store = {
    get value(): string {
        return "ready";
    },
    set value(next: string | int32): void {},
} as Store;

=== dir ===
interface Store {
/// @generic.template symbol=Store parameters=(this: Store)
/// @type.symbol symbol=Store type=Store
/// @definition.interface symbol=Store template=(this: Store)
/// @definition.where symbol=Store relation=satisfies left=this right=Store
/// @definition.method symbol=Store.value#1 source="get value(): string" slot=value role=getter type=() => string
/// @definition.method symbol=Store.value#2 source="set value(next: string | int32)" slot=value role=setter type=(string | int32) => void

    get value(): string;
    /// @type.symbol symbol=Store.value#1 source="get value(): string" type=() => string

    set value(next: string | int32);
    /// @type.symbol symbol=Store.value#2 source="set value(next: string | int32)" type=(string | int32) => void
    /// @type.symbol symbol=Store.value.next source="next: string | int32" type=string | int32

}
const store: Store = {
/// @type.symbol symbol=store source=store type=Store
/// @resolution.pattern source=store kind=binding target=store
/// @resolution.name source=Store target=Store
/// @type.node type={ get value(): string; set value(value: string | int32) }

    get value(): string { return "ready"; },
    /// @type.symbol symbol=symbol7 source="get value(): string { return \"ready\"; }" type=() => string
    /// @type.node source="\"ready\"" type="ready"

    set value(next: string | int32): void {},
    /// @type.symbol symbol=symbol8 source="set value(next: string | int32): void {}" type=(string | int32) => void

};
"#,
    );
}

/// Reject an object getter incompatible with its property expectation.
#[test]
fn test_reject_an_object_getter_incompatible_with_its_property_expectation() {
    let session = TestSession::single(
        r#"
interface Store {
    get value(): string;
}
const store: Store = {
    get value(): int32 { return 1; },
};
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Store {
    get value(): string;
}
const store: Store = {
    get value(): int32 {
        return 1;
    },
};

=== dir ===
interface Store {
/// @generic.template symbol=Store parameters=(this: Store)
/// @type.symbol symbol=Store type=Store
/// @definition.interface symbol=Store template=(this: Store)
/// @definition.where symbol=Store relation=satisfies left=this right=Store
/// @definition.method symbol=Store.value source="get value(): string" slot=value role=getter type=() => string

    get value(): string;
    /// @type.symbol symbol=Store.value source="get value(): string" type=() => string

}
const store: Store = {
/// @type.symbol symbol=store source=store type=Store
/// @resolution.pattern source=store kind=binding target=store
/// @resolution.name source=Store target=Store

    get value(): int32 { return 1; },
    /// @type.symbol symbol=symbol4 source="get value(): int32 { return 1; }" type=() => int32

};
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ readonly value: int32 }' is not assignable to type 'Store'"
/// @diagnostic.label line=5 column=22 span="{\n    get value(): int32 { return 1; },\n}" line_source="const store: Store = {"
/// @diagnostic.related line=5 column=14 span="Store" line_source="const store: Store = {" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'value': expected 'string', found 'int32'"
"#,
    );
}

/// Reject an object setter incompatible with its property expectation.
#[test]
fn test_reject_an_object_setter_incompatible_with_its_property_expectation() {
    let session = TestSession::single(
        r#"
interface Store {
    set value(next: string);
}
const store: Store = {
    set value(next: int32): void {},
};
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Store {
    set value(next: string);
}
const store: Store = {
    set value(next: int32): void {},
};

=== dir ===
interface Store {
/// @generic.template symbol=Store parameters=(this: Store)
/// @type.symbol symbol=Store type=Store
/// @definition.interface symbol=Store template=(this: Store)
/// @definition.where symbol=Store relation=satisfies left=this right=Store
/// @definition.method symbol=Store.value source="set value(next: string)" slot=value role=setter type=(string) => void

    set value(next: string);
    /// @type.symbol symbol=Store.value source="set value(next: string)" type=(string) => void
    /// @type.symbol symbol=Store.value.next source="next: string" type=string

}
const store: Store = {
/// @type.symbol symbol=store source=store type=Store
/// @resolution.pattern source=store kind=binding target=store
/// @resolution.name source=Store target=Store

    set value(next: int32): void {},
    /// @type.symbol symbol=symbol5 source="set value(next: int32): void {}" type=(int32) => void

};
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ set value(value: int32) }' is not assignable to type 'Store'"
/// @diagnostic.label line=5 column=22 span="{\n    set value(next: int32): void {},\n}" line_source="const store: Store = {"
/// @diagnostic.related line=5 column=14 span="Store" line_source="const store: Store = {" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'value': expected 'int32', found 'string'"
"#,
    );
}

/// Reject object accessors missing the operation required by their interfaces.
#[test]
fn test_reject_object_accessors_missing_a_required_property_operation() {
    let session = TestSession::single(
        r#"
interface Readable {
    get value(): string;
}
interface Writable {
    set value(next: string);
}

const readable: Readable = {
    set value(next: string): void {},
};
const writable: Writable = {
    get value(): string { return "ready"; },
};
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Readable {
    get value(): string;
}
interface Writable {
    set value(next: string);
}

const readable: Readable = {
    set value(next: string): void {},
};
const writable: Writable = {
    get value(): string {
        return "ready";
    },
};

=== dir ===
interface Readable {
/// @generic.template symbol=Readable parameters=(this: Readable)
/// @type.symbol symbol=Readable type=Readable
/// @definition.interface symbol=Readable template=(this: Readable)
/// @definition.where symbol=Readable relation=satisfies left=this right=Readable
/// @definition.method symbol=Readable.value source="get value(): string" slot=value role=getter type=() => string

    get value(): string;
    /// @type.symbol symbol=Readable.value source="get value(): string" type=() => string

}
interface Writable {
/// @generic.template symbol=Writable parameters=(this: Writable)
/// @type.symbol symbol=Writable type=Writable
/// @definition.interface symbol=Writable template=(this: Writable)
/// @definition.where symbol=Writable relation=satisfies left=this right=Writable
/// @definition.method symbol=Writable.value source="set value(next: string)" slot=value role=setter type=(string) => void

    set value(next: string);
    /// @type.symbol symbol=Writable.value source="set value(next: string)" type=(string) => void
    /// @type.symbol symbol=Writable.value.next source="next: string" type=string

}

const readable: Readable = {
/// @type.symbol symbol=readable source=readable type=Readable
/// @resolution.pattern source=readable kind=binding target=readable
/// @resolution.name source=Readable target=Readable

    set value(next: string): void {},
    /// @type.symbol symbol=symbol8 source="set value(next: string): void {}" type=(string) => void

};
const writable: Writable = {
/// @type.symbol symbol=writable source=writable type=Writable
/// @resolution.pattern source=writable kind=binding target=writable
/// @resolution.name source=Writable target=Writable

    get value(): string { return "ready"; },
    /// @type.symbol symbol=symbol11 source="get value(): string { return \"ready\"; }" type=() => string

};
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ set value(value: string) }' is not assignable to type 'Readable'"
/// @diagnostic.label line=9 column=28 span="{\n    set value(next: string): void {},\n}" line_source="const readable: Readable = {"
/// @diagnostic.related line=9 column=17 span="Readable" line_source="const readable: Readable = {" message="expected due to this annotation"
/// @diagnostic.error id=not-assignable message="type '{ readonly value: string }' is not assignable to type 'Writable'"
/// @diagnostic.label line=12 column=28 span="{\n    get value(): string { return \"ready\"; },\n}" line_source="const writable: Writable = {"
/// @diagnostic.related line=12 column=17 span="Writable" line_source="const writable: Writable = {" message="expected due to this annotation"
"#,
    );
}

/// Reject two getters for one object property.
#[test]
fn test_reject_duplicate_object_getters() {
    let session = TestSession::single(
        r#"
const store = {
    get value(): string { return "ready"; },
    get value(): string { return "waiting"; },
};
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const store: { readonly value: string } = {
    get value(): string {
        return "ready";
    },
    get value(): string {
        return "waiting";
    },
};

=== dir ===
const store = {
/// @type.symbol symbol=store source=store type={ readonly value: string }
/// @resolution.pattern source=store kind=binding target=store

    get value(): string { return "ready"; },
    /// @type.symbol symbol=symbol1 source="get value(): string { return \"ready\"; }" type=() => string

    get value(): string { return "waiting"; },
    /// @type.symbol symbol=symbol2 source="get value(): string { return \"waiting\"; }" type=() => string

};
"#,
        r#"
/// @diagnostic.error id=duplicate-member message="member 'value' is already declared"
/// @diagnostic.label line=4 column=9 span="value" line_source="get value(): string { return \"waiting\"; },"
"#,
    );
}

/// Reject two setters for one object property.
#[test]
fn test_reject_duplicate_object_setters() {
    let session = TestSession::single(
        r#"
const store = {
    set value(next: string): void {},
    set value(next: string): void {},
};
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const store: { value: string } = {
    set value(next: string): void {},
    set value(next: string): void {},
};

=== dir ===
const store = {
/// @type.symbol symbol=store source=store type={ set value(value: string) }
/// @resolution.pattern source=store kind=binding target=store

    set value(next: string): void {},
    /// @type.symbol symbol=symbol1 source="set value(next: string): void {}" type=(string) => void

    set value(next: string): void {},
    /// @type.symbol symbol=symbol3 source="set value(next: string): void {}" type=(string) => void

};
"#,
        r#"
/// @diagnostic.error id=duplicate-member message="member 'value' is already declared"
/// @diagnostic.label line=4 column=9 span="value" line_source="set value(next: string): void {},"
"#,
    );
}

/// Reject an accessor colliding with an object field.
#[test]
fn test_reject_object_accessor_field_collision() {
    let session = TestSession::single(
        r#"
const store = {
    value: "ready",
    get value(): string { return "waiting"; },
};
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const store: { readonly value: string } = {
    value: "ready",
    get value(): string {
        return "waiting";
    },
};

=== dir ===
const store = {
/// @type.symbol symbol=store source=store type={ readonly value: string }
/// @resolution.pattern source=store kind=binding target=store

    value: "ready",
    get value(): string { return "waiting"; },
    /// @type.symbol symbol=symbol1 source="get value(): string { return \"waiting\"; }" type=() => string

};
"#,
        r#"
/// @diagnostic.error id=duplicate-member message="member 'value' is already declared"
/// @diagnostic.label line=4 column=9 span="value" line_source="get value(): string { return \"waiting\"; },"
"#,
    );
}

/// Type a closure member of an object literal from the contextual member signature.
#[test]
fn test_type_a_member_closure_from_the_contextual_object_type() {
    let session = TestSession::single(
        r#"
type Handlers = { onCount: (value: int32) => void };

const handlers: Handlers = {
    onCount: (value) => {
        value;
    },
};
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Handlers = { onCount: (value: int32) => void };

const handlers: Handlers = {
    onCount: (value: int32): void => {
        value;
    },
};

=== dir ===
type Handlers = { onCount: (value: int32) => void };
/// @type.symbol symbol=Handlers source="type Handlers = { onCount: (value: int32) => void }" type={ onCount: (int32) => void }
/// @definition.type symbol=Handlers source="type Handlers = { onCount: (value: int32) => void }" value={ onCount: (int32) => void }
/// @type.symbol symbol=Handlers.onCount source="onCount: (value: int32) => void" type=(int32) => void
/// @type.symbol symbol=Handlers.value source="value: int32" type=int32

const handlers: Handlers = {
/// @type.symbol symbol=handlers source=handlers type=Handlers
/// @resolution.pattern source=handlers kind=binding target=handlers
/// @resolution.name source=Handlers target=Handlers
/// @type.node type={ onCount: (int32) => void }

    onCount: (value) => {
    /// @type.symbol symbol=symbol5 type=Function<(int32,), void, "readonly">
    /// @type.node type=Function<(int32,), void, "readonly">
    /// @type.symbol symbol=symbol5.value source=value type=int32

        value;
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=symbol5.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=symbol5.value

    },
};
"#,
        r#"

"#,
    );
}

/// Spread a generic reduce accumulator typed through the callback's contextual signature.
#[test]
fn test_spread_the_accumulator_of_a_generic_reduce() {
    let session = TestSession::single(
        r#"
function retain(source: boolean[]): { active: boolean } {
    return source.reduce<{ active: boolean }>(
        (output, value) => ({ ...output, active: value }),
        { active: false },
    );
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function retain(source: boolean[]): { active: boolean } {
    return source.reduce<{ active: boolean }>(
        (output: { active: boolean }, value: boolean): { active: boolean } => ({
            ...output,
            active: value,
        }),
        { active: false },
    );
}

=== dir ===
function retain(source: boolean[]): { active: boolean } {
/// @type.symbol symbol=retain type=(boolean[]) => { active: boolean }
/// @type.symbol symbol=retain.source source="source: boolean[]" type=boolean[]
/// @type.symbol symbol=retain.active#1 source="active: boolean" type=boolean

    return source.reduce<{ active: boolean }>(
    /// @resolution.name source=source target=retain.source
    /// @resolution.member source=source.reduce receiver=boolean[] type=<reduce.U#2, reduce#2.'a>(this: &reduce#2.'a readonly boolean[], (reduce.U#2, boolean, isize) => reduce.U#2, reduce.U#2) => reduce.U#2 kind=symbol target_receiver=boolean[] target=reduce#2
    /// @resolution.call parameters=(({ active: boolean }, boolean, isize) => { active: boolean }, { active: boolean }) arguments=(provided((output, value) => ({ ...output, active: value })) as ({ active: boolean }, boolean, isize) => { active: boolean }, provided({ active: false }) as { active: boolean }) return={ active: boolean } regions=("managed" & "local") kind=symbol target=reduce#2 receiver=boolean[] adjustments=(borrow(&'managed readonly boolean[])) instance="Array<boolean>.<extension#4>.reduce#2<{ active: boolean }, \"managed\" & \"local\">"
    /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=source root=retain.source
    /// @generic.instantiation id="reduce#2<boolean, { active: boolean }, \"managed\" & \"local\">" template=reduce#2 arguments=(boolean, { active: boolean }, "managed" & "local")
    /// @generic.instantiation id=reduce#2<boolean> template=reduce#2 arguments=(boolean)
    /// @type.symbol symbol=retain.active#2 source="active: boolean" type=boolean

        (output, value) => ({ ...output, active: value }),
        /// @type.symbol symbol=retain.symbol7 source=(output, value) => ({ ...output, active: value }) type=Function<({ active: boolean }, boolean), { active: boolean }, "readonly">
        /// @type.symbol symbol=retain.symbol7.output source=output type={ active: boolean }
        /// @type.symbol symbol=retain.symbol7.value source=value type=boolean
        /// @resolution.name source=output target=retain.symbol7.output
        /// @resolution.access source=output root=retain.symbol7.output
        /// @resolution.name source=value target=retain.symbol7.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=retain.symbol7.value

        { active: false },
    );
}
"#,
        r#"

"#,
    );
}

/// Spread an unannotated closure parameter whose type stays open.
#[test]
fn test_spread_an_open_closure_parameter() {
    let session = TestSession::single(
        r#"
const merge = (input) => ({ ...input, active: true });
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const merge = (input) => ({ ...input, active: true });

=== dir ===
const merge = (input) => ({ ...input, active: true });
/// @type.symbol symbol=merge source=merge type=Function<(<error>,), <error>, "readonly">
/// @resolution.pattern source=merge kind=binding target=merge
/// @type.symbol symbol=symbol1 source=(input) => ({ ...input, active: true }) type=Function<(<error>,), <error>, "readonly">
/// @type.symbol symbol=symbol1.input source=input type=<error>
/// @resolution.poisoned source={ ...input, active: true }
/// @resolution.name source=input target=symbol1.input
/// @resolution.access source=input root=symbol1.input
"#,
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=2 column=27 span="{ ...input, active: true }" line_source="const merge = (input) => ({ ...input, active: true });"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}
