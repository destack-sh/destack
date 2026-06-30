use crate::tests::{DirRows, TestSession};

#[test]
fn test_struct_tagged_literal_constructs_value() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
point satisfies Point;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2 };
point satisfies Point;

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
/// @resolution.name source=Point target=Point

point satisfies Point;
/// @resolution.name source=point target=point
/// @resolution.name source=Point target=Point
"#,
    );
}

#[test]
fn test_struct_tagged_literal_exposes_methods() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

const next = Counter { value: 1 }.increment();
next satisfies Counter;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

const next: Counter = (Counter { value: 1 }).increment();
next satisfies Counter;

=== checked ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.increment slot=increment type=(this: Counter) => Counter

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    increment(): Counter {
    /// @type.symbol symbol=Counter.increment type=(this: Counter) => Counter
    /// @resolution.name source=Counter target=Counter

        Counter { value: this.value + 1 }
        /// @resolution.name source=Counter target=Counter
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
        /// @resolution.call source="this.value + 1" parameters=() return=int32 kind=builtin builtin=binary.add
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter

    }
}

const next = Counter { value: 1 }.increment();
/// @type.symbol symbol=next source=next type=Counter
/// @resolution.name source=Counter target=Counter
/// @resolution.member source="Counter { value: 1 }.increment" receiver=Counter kind=symbol target=Counter.increment
/// @resolution.call source="Counter { value: 1 }.increment()" parameters=() return=Counter kind=symbol target=Counter.increment receiver=Counter

next satisfies Counter;
/// @resolution.name source=next target=next
/// @resolution.name source=Counter target=Counter
"#,
    );
}

#[test]
fn test_struct_tagged_literal_requires_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1 };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1 };

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

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error code=EC215 message="missing required property 'y' for type 'Point'"
/// @diagnostic.label line=7 column=15 span="Point { x: 1 }" line_source="const point = Point { x: 1 };"
"#,
    );
}

#[test]
fn test_struct_tagged_literal_rejects_extra_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2, z: 3 };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2, z: 3 };

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

const point = Point { x: 1, y: 2, z: 3 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'z' in object literal for type 'Point'"
/// @diagnostic.label line=7 column=15 span="Point { x: 1, y: 2, z: 3 }" line_source="const point = Point { x: 1, y: 2, z: 3 };"
"#,
    );
}

#[test]
fn test_struct_rejects_new_constructor_syntax() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = new Point(1, 2);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point = new Point(1, 2);

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

const point = new Point(1, 2);
/// @type.symbol symbol=point source=point type=<error>
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error code=EC313 message="type 'Point' cannot be constructed with 'new'; construct value types with 'T { … }'"
/// @diagnostic.label line=7 column=15 span="new Point(1, 2)" line_source="const point = new Point(1, 2);"
"#,
    );
}

#[test]
fn test_struct_methods_mutate_fields() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    increment(): int32 {
        this.value = this.value + 1;
        this.value
    }
}

let counter = Counter { value: 1 };
const next = counter.increment();
next satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;

    increment(): int32 {
        this.value = this.value + 1;
        this.value
    }
}

let counter: Counter = Counter { value: 1 };
const next: int32 = counter.increment();
next satisfies int32;

=== checked ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.increment slot=increment type=(this: Counter) => int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    increment(): int32 {
    /// @type.symbol symbol=Counter.increment type=(this: Counter) => int32

        this.value = this.value + 1;
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.pattern.assign source=this.value kind=place place=field(Counter.value) type=int32
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
        /// @resolution.call source="this.value + 1" parameters=() return=int32 kind=builtin builtin=binary.add
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter

        this.value
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter

    }
}

let counter = Counter { value: 1 };
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.name source=Counter target=Counter

const next = counter.increment();
/// @type.symbol symbol=next source=next type=int32
/// @resolution.name source=counter target=counter
/// @resolution.member source=counter.increment receiver=Counter kind=symbol target=Counter.increment
/// @resolution.call source=counter.increment() parameters=() return=int32 kind=symbol target=Counter.increment receiver=Counter

next satisfies int32;
/// @resolution.name source=next target=next
"#,
    );
}

#[test]
fn test_object_literal_does_not_construct_struct() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;
}

const counter: Counter = { value: 1 };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;
}

const counter: Counter = { value: 1 };

=== checked ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

}

const counter: Counter = { value: 1 };
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.name source=Counter target=Counter
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '{ value: 1 }' is not assignable to type 'Counter'"
/// @diagnostic.label line=6 column=26 span="{ value: 1 }" line_source="const counter: Counter = { value: 1 };"
"#,
    );
}
