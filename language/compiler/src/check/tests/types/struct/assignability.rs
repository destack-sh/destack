use crate::tests::{DirRows, TestSession};

#[test]
fn test_struct_satisfies_structural_interface() {
    let session = TestSession::single(
        r#"
interface HasX {
    x: int32;
}

struct Point {
    x: int32;
}

const value: HasX = Point { x: 1 };
value satisfies HasX;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface HasX {
    x: int32;
}

struct Point {
    x: int32;
}

const value: HasX = Point { x: 1 } as HasX;
value satisfies HasX;

=== checked ===
interface HasX {
/// @type.symbol symbol=HasX type=HasX
/// @definition.field symbol=HasX.x source="x: int32" key=x type=int32
/// @definition.interface symbol=HasX

    x: int32;
    /// @type.symbol symbol=HasX.x source="x: int32" type=int32

}

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const value: HasX = Point { x: 1 };
/// @type.symbol symbol=value source=value type=HasX
/// @resolution.name source=HasX target=HasX
/// @resolution.name source=Point target=Point

value satisfies HasX;
/// @resolution.name source=value target=value
/// @resolution.name source=HasX target=HasX
"#,
    );
}

#[test]
fn test_struct_satisfies_structural_object_type() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const point = Point { x: 1 };
const value: { readonly x: int32 } = point;
value satisfies { readonly x: int32 };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const point: { readonly x: int32 } = Point { x: 1 } as { readonly x: int32 };
const value: { readonly x: int32 } = point;
value satisfies { readonly x: int32 };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type={ readonly x: int32 }
/// @resolution.name source=Point target=Point

const value: { readonly x: int32 } = point;
/// @type.symbol symbol=value source=value type={ readonly x: int32 }
/// @resolution.name source=point target=point

value satisfies { readonly x: int32 };
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_struct_satisfies_optional_interface_fields() {
    let session = TestSession::single(
        r#"
interface HasCount {
    count?: int32;
}

struct Counter {
    count: int32;
}

const counter: HasCount = Counter { count: 1 };
counter satisfies HasCount;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface HasCount {
    count?: int32;
}

struct Counter {
    count: int32;
}

const counter: HasCount = Counter { count: 1 } as HasCount;
counter satisfies HasCount;

=== checked ===
interface HasCount {
/// @type.symbol symbol=HasCount type=HasCount
/// @definition.field symbol=HasCount.count source="count?: int32" key=count type=int32 | undefined
/// @definition.interface symbol=HasCount

    count?: int32;
    /// @type.symbol symbol=HasCount.count source="count?: int32" type=int32 | undefined

}

struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.field symbol=Counter.count source="count: int32" key=count type=int32
/// @definition.struct symbol=Counter

    count: int32;
    /// @type.symbol symbol=Counter.count source="count: int32" type=int32

}

const counter: HasCount = Counter { count: 1 };
/// @type.symbol symbol=counter source=counter type=HasCount
/// @resolution.name source=HasCount target=HasCount
/// @resolution.name source=Counter target=Counter

counter satisfies HasCount;
/// @resolution.name source=counter target=counter
/// @resolution.name source=HasCount target=HasCount
"#,
    );
}

#[test]
fn test_struct_implements_clause_requires_members() {
    let session = TestSession::single(
        r#"
interface Drawable {
    draw(): void;
}

struct Point implements Drawable {
    x: int32;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Drawable {
    draw(): void;
}

struct Point implements Drawable {
    x: int32;
}

=== checked ===
interface Drawable {
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=(this: Drawable) => void

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=(this: Drawable) => void

}

struct Point implements Drawable {
/// @type.symbol symbol=Point type=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.struct symbol=Point
/// @definition.implements symbol=Point source=Drawable target=Drawable
/// @resolution.name source=Drawable target=Drawable

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}
"#,
        r#"
/// @diagnostic.error code=EC203 message="type 'Point' does not implement interface 'Drawable'"
/// @diagnostic.label line=6 column=25 source="struct Point implements Drawable {"
"#,
    );
}

#[test]
fn test_struct_implements_rejects_conflicting_generic_heritage() {
    let session = TestSession::single(
        r#"
interface Base<T> {}
interface Left extends Base<string> {}
interface Right extends Base<int32> {}

struct Point implements Left, Right {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Base<T> {}
interface Left extends Base<string> {}
interface Right extends Base<int32> {}

struct Point implements Left, Right {}

=== checked ===
interface Base<T> {}
/// @generic.template symbol=Base parameters=(T)
/// @type.symbol symbol=Base source="interface Base<T> {}" type=Base<T>
/// @definition.interface symbol=Base source="interface Base<T> {}" template=LocalGenericTemplateId(0)
/// @type.symbol symbol=Base.T source=T type=T

interface Left extends Base<string> {}
/// @type.symbol symbol=Left source="interface Left extends Base<string> {}" type=Left
/// @definition.interface symbol=Left source="interface Left extends Base<string> {}"
/// @definition.extends symbol=Left source=Base<string> target=Base arguments=(string)
/// @resolution.name source=Base target=Base

interface Right extends Base<int32> {}
/// @type.symbol symbol=Right source="interface Right extends Base<int32> {}" type=Right
/// @definition.interface symbol=Right source="interface Right extends Base<int32> {}"
/// @definition.extends symbol=Right source=Base<int32> target=Base arguments=(int32)
/// @resolution.name source=Base target=Base

struct Point implements Left, Right {}
/// @type.symbol symbol=Point source="struct Point implements Left, Right {}" type=Point
/// @definition.struct symbol=Point source="struct Point implements Left, Right {}"
/// @definition.implements symbol=Point source=Left target=Left
/// @definition.implements symbol=Point source=Right target=Right
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right
"#,
        r#"
/// @diagnostic.error code=EC617 message="type 'Point' has conflicting heritage for 'Base'"
/// @diagnostic.label line=6 column=31 source="struct Point implements Left, Right {}"
"#,
    );
}

#[test]
fn test_struct_does_not_satisfy_class_by_shape() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

class PointClass {
    x: int32 = 0;
}

const point = Point { x: 1 };
const value: PointClass = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

class PointClass {
    x: int32 = 0;
}

const point: Point = Point { x: 1 };
const value: PointClass = point;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

class PointClass {
/// @type.symbol symbol=PointClass type=PointClass
/// @definition.field symbol=PointClass.x source="x: int32 = 0" key=x type=int32
/// @definition.class symbol=PointClass

    x: int32 = 0;
    /// @type.symbol symbol=PointClass.x source="x: int32 = 0" type=int32

}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.name source=Point target=Point

const value: PointClass = point;
/// @type.symbol symbol=value source=value type=PointClass
/// @resolution.name source=PointClass target=PointClass
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Point' is not assignable to type 'PointClass'"
/// @diagnostic.label line=10 column=7 source="const point = Point { x: 1 };"
"#,
    );
}

#[test]
fn test_class_does_not_satisfy_struct_by_shape() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

class PointClass {
    x: int32 = 0;
}

const point = new PointClass();
const value: Point = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

class PointClass {
    x: int32 = 0;
}

const point: Point = new PointClass();
const value: Point = point;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

class PointClass {
/// @type.symbol symbol=PointClass type=PointClass
/// @definition.field symbol=PointClass.x source="x: int32 = 0" key=x type=int32
/// @definition.class symbol=PointClass

    x: int32 = 0;
    /// @type.symbol symbol=PointClass.x source="x: int32 = 0" type=int32

}

const point = new PointClass();
/// @type.symbol symbol=point source=point type=Point
/// @resolution.construct source="new PointClass()" parameters=() return=PointClass kind=class target=PointClass
/// @resolution.name source=PointClass target=PointClass

const value: Point = point;
/// @type.symbol symbol=value source=value type=Point
/// @resolution.name source=Point target=Point
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'PointClass' is not assignable to type 'Point'"
/// @diagnostic.label line=10 column=15 source="const point = new PointClass();"
"#,
    );
}
