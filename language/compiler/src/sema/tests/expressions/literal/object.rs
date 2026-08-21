use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_literal_infers_property_types() {
    let session = TestSession::single(
        r#"
const value = { a: 1, b: "two" };
"#,
    );

    session.assert_dir(
        "main.ds",
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
fn test_object_literal_uses_shorthand_binding_types() {
    let session = TestSession::single(
        r#"
const name = "Ada";
const age = 42;
const person = { name, age };
"#,
    );

    session.assert_dir(
        "main.ds",
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
/// @resolution.place source=name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=name root=name
/// @type.node source=age type=42
/// @resolution.name source=age target=age
/// @resolution.place source=age placement="local" lifetime="static" access="readonly"
/// @resolution.access source=age root=age
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

    session.assert_dir(
        "main.ds",
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
fn test_const_asserted_object_preserves_literal_properties() {
    let session = TestSession::single(
        r#"
const value = { a: 1, b: "two" } as const;
"#,
    );

    session.assert_dir(
        "main.ds",
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
fn test_contextual_object_literal_rejects_property_mismatch() {
    let session = TestSession::single(
        r#"
const value: { a: number; b: string } = { a: 1, b: 2 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: { a: float64; b: string } = { a: 1, b: 2 };

=== dir ===
const value: { a: number; b: string } = { a: 1, b: 2 };
/// @type.symbol symbol=value source=value type={ a: float64; b: string }
/// @resolution.pattern source=value kind=binding target=value
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
fn test_contextual_object_literal_contextualizes_empty_array_field() {
    let session = TestSession::single(
        r#"
const state: { reactions: int32[] } = { reactions: [] };
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const state: { reactions: int32[] } = { reactions: [] };

=== dir ===
const state: { reactions: int32[] } = { reactions: [] };
/// @type.symbol symbol=state source=state type={ reactions: int32[] }
/// @resolution.pattern source=state kind=binding target=state
/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=collections.slice.new<memory.init.MaybeUninit<int32>> template=collections.slice.new arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.init.MaybeUninit<int32> template=memory.init.MaybeUninit arguments=(int32)
/// @type.node source={ reactions: [] } type={ reactions: int32[] }
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(&collections.array.arrayFromSlice.'a readonly Slice<collections.array.arrayFromSlice.T>) arguments=(rest() as int32) return=int32[] kind=symbol target=collections.array.arrayFromSlice instance=collections.array.arrayFromSlice<int32>
/// @generic.instantiation id=collections.array.arrayFromSlice<int32> template=collections.array.arrayFromSlice arguments=(int32)
/// @generic.instance id=collections.array.arrayFromSlice<int32> template=collections.array.arrayFromSlice arguments=(int32)
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

    session.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            ,
        r#"
=== annotated ===
type Counts = { [key: string]: int32 };

const counts: Counts = { apples: 1, oranges: 2 };

=== dir ===
type Counts = { [key: string]: int32 };
/// @type.symbol symbol=Counts source="type Counts = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Counts source="type Counts = { [key: string]: int32 }" value={ [key: string]: int32 }

const counts: Counts = { apples: 1, oranges: 2 };
/// @type.symbol symbol=counts source=counts type={ [key: string]: int32 }
/// @resolution.pattern source=counts kind=binding target=counts
/// @resolution.name source=Counts target=Counts
/// @type.node source={ apples: 1, oranges: 2 } type={ apples: int32; oranges: int32 }
/// @coercion.node source={ apples: 1, oranges: 2 } from={ apples: int32; oranges: int32 } adjustments=[{ kind: erase, target: { [key: string]: int32 } }] origin=implicit
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: int32 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: int32 }] origin=implicit
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

    session.assert_dir(
        "main.ds",
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
fn test_object_spread_adds_fields() {
    let session = TestSession::single(
        r#"
const base = { a: 1, b: "two" };
const value = { ...base, c: true };
"#,
    );

    session.assert_dir(
        "main.ds",
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
fn test_object_spread_overrides_fields() {
    let session = TestSession::single(
        r#"
const base = { a: 1, b: 2 };
const value = { ...base, b: "two" };
"#,
    );

    session.assert_dir(
        "main.ds",
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

    session.assert_dir(
        "main.ds",
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
/// @resolution.place source=point placement="local" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
/// @type.node source=3 type=3
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

    session.assert_dir(
        "main.ds",
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

    session.assert_dir(
        "main.ds",
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
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

=== dir ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(string) => User

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=User.constructor type=(string) => User
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string

        this.name = name;
        /// @type.node source="this.name = name" type=string
        /// @type.node source=this type=User
        /// @type.node source=this.name type=string
        /// @resolution.receiver source=this kind=this declaration=User type=User
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.access source=this.name root=this keys=[name]
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
        "main.ds",
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
fn test_object_getter_infers_read_property() {
    let session = TestSession::single(
        r#"
const store = {
    get value(): string { return "ready"; },
};
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
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

/// @check.stats.solve variables=0 constraints=0 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

/// Infer a setter as a writable object property.
#[test]
fn test_object_setter_infers_write_property() {
    let session = TestSession::single(
        r#"
const store = {
    set value(next: string): void {},
};
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
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
    /// @type.symbol symbol=symbol1.next source="next: string" type=string

};

/// @check.stats.solve variables=0 constraints=0 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

/// Merge one getter and setter into one readable and writable object property.
#[test]
fn test_object_accessor_pair_infers_property_operations() {
    let session = TestSession::single(
        r#"
const store = {
    get value(): string { return "ready"; },
    set value(next: string | int32): void {},
};
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
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
    /// @type.symbol symbol=symbol2.next source="next: string | int32" type=string | int32

};

/// @check.stats.solve variables=0 constraints=0 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

/// Accept an object accessor pair under a structural property expectation.
#[test]
fn test_object_accessor_pair_satisfies_property_expectation() {
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
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
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
/// @type.symbol symbol=Store type=Store
/// @definition.interface symbol=Store
/// @definition.method symbol=Store.value#1 source="get value(): string" slot=value role=getter type=(this: Store) => string
/// @definition.method symbol=Store.value#2 source="set value(next: string | int32)" slot=value role=setter type=(this: Store, string | int32) => void

    get value(): string;
    /// @type.symbol symbol=Store.value#1 source="get value(): string" type=(this: Store) => string

    set value(next: string | int32);
    /// @type.symbol symbol=Store.value#2 source="set value(next: string | int32)" type=(this: Store, string | int32) => void
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
    /// @type.symbol symbol=symbol8.next source="next: string | int32" type=string | int32

};

/// @check.stats.solve variables=0 constraints=0 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

/// Reject an object getter incompatible with its property expectation.
#[test]
fn test_object_getter_rejects_incompatible_property_expectation() {
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

    session.assert_dir_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=not-assignable message="type '{ readonly value: int32 }' is not assignable to type 'Store'"
/// @diagnostic.label line=5 column=22 span="{\n    get value(): int32 { return 1; },\n}" line_source="const store: Store = {"
/// @diagnostic.related line=5 column=14 span="Store" line_source="const store: Store = {" message="expected due to this annotation"
"#,
    );
}

/// Reject an object setter incompatible with its property expectation.
#[test]
fn test_object_setter_rejects_incompatible_property_expectation() {
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

    session.assert_dir_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=not-assignable message="type '{ set value(value: int32) }' is not assignable to type 'Store'"
/// @diagnostic.label line=5 column=22 span="{\n    set value(next: int32): void {},\n}" line_source="const store: Store = {"
/// @diagnostic.related line=5 column=14 span="Store" line_source="const store: Store = {" message="expected due to this annotation"
"#,
    );
}

/// Reject object accessors missing the operation required by their interfaces.
#[test]
fn test_object_accessor_rejects_missing_property_operation() {
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

    session.assert_dir_diagnostics(
        "main.ds",
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
        "main.ds",
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
        "main.ds",
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
    /// @type.symbol symbol=symbol1.next source="next: string" type=string

    set value(next: string): void {},
    /// @type.symbol symbol=symbol3 source="set value(next: string): void {}" type=(string) => void
    /// @type.symbol symbol=symbol3.next source="next: string" type=string

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
        "main.ds",
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
