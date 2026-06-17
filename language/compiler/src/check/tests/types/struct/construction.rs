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

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32
}

const point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point type=Point
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

const next: Counter = Counter { value: 1 }.increment();
next satisfies Counter;

=== checked ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    increment(): Counter {
    /// @type.symbol symbol=Counter.increment type=(this: Counter) => Counter
    /// @resolution.name source=Counter target=Counter

        Counter { value: this.value + 1 }
        /// @resolution.name source=Counter target=Counter
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
    }
}

const next = Counter { value: 1 }.increment();
/// @type.symbol symbol=next type=Counter
/// @resolution.name source=Counter target=Counter
/// @resolution.member source="Counter { value: 1 }.increment" receiver=Counter kind=symbol target=Counter.increment

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

const point = Point { x: 1 };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32
}

const point = Point { x: 1 };
/// @type.symbol symbol=point type=<error>
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error code=EC215 message="missing required property 'y' for type 'Point'"
/// @diagnostic.label line=7 column=15 source="const point = Point { x: 1 };"
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

const point = Point { x: 1, y: 2, z: 3 };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32
}

const point = Point { x: 1, y: 2, z: 3 };
/// @type.symbol symbol=point type=<error>
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'z' in object literal for type 'Point'"
/// @diagnostic.label line=7 column=34 source="const point = Point { x: 1, y: 2, z: 3 };"
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

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32
}

const point = new Point(1, 2);
/// @type.symbol symbol=point type=<error>
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error code=EC300 message="type 'Point' is not constructible with 'new'"
/// @diagnostic.label line=7 column=15 source="const point = new Point(1, 2);"
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

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    increment(): int32 {
    /// @type.symbol symbol=Counter.increment type=(this: Counter) => int32

        this.value = this.value + 1;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value

        this.value
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
    }
}

let counter = Counter { value: 1 };
/// @type.symbol symbol=counter type=Counter
/// @resolution.name source=Counter target=Counter

const next = counter.increment();
/// @type.symbol symbol=next type=int32
/// @resolution.name source=counter target=counter
/// @resolution.member source=counter.increment receiver=Counter kind=symbol target=Counter.increment

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

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32
}

const counter: Counter = { value: 1 };
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.name source=Counter target=Counter
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '{ value: 1 }' is not assignable to type 'Counter'"
/// @diagnostic.label line=6 column=7 source="const counter: Counter = { value: 1 };"
"#,
    );
}
