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

const value: HasX = Point { x: 1 };
value satisfies HasX;

=== checked ===
interface HasX {
/// @type.symbol symbol=HasX type=HasX
/// @definition.interface symbol=HasX

    x: int32;
    /// @type.symbol symbol=HasX.x source="x: int32" type=int32
}

struct Point {
/// @type.symbol symbol=Point type=Point
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

const point: Point = Point { x: 1 };
const value: { readonly x: int32 } = point;
value satisfies { readonly x: int32 };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32
}

const point = Point { x: 1 };
/// @type.symbol symbol=point type=Point
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
interface HasName {
    name?: string;
}

struct Person {
    name: string;
}

const person: HasName = Person { name: "Ada" };
person satisfies HasName;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface HasName {
    name?: string;
}

struct Person {
    name: string;
}

const person: HasName = Person { name: "Ada" };
person satisfies HasName;

=== checked ===
interface HasName {
/// @type.symbol symbol=HasName type=HasName
/// @definition.interface symbol=HasName

    name?: string;
    /// @type.symbol symbol=HasName.name source="name?: string" type=string | undefined
}

struct Person {
/// @type.symbol symbol=Person type=Person
/// @definition.struct symbol=Person

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string
}

const person: HasName = Person { name: "Ada" };
/// @type.symbol symbol=person source=person type=HasName
/// @resolution.name source=HasName target=HasName
/// @resolution.name source=Person target=Person

person satisfies HasName;
/// @resolution.name source=person target=person
/// @resolution.name source=HasName target=HasName
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

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=() => void
}

struct Point implements Drawable {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @resolution.name source=Drawable target=Drawable

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32
}
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Point' is not assignable to type 'Drawable'"
/// @diagnostic.label line=6 column=8 source="struct Point implements Drawable {"
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
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32
}

class PointClass {
/// @type.symbol symbol=PointClass type=PointClass

    x: int32 = 0;
    /// @type.symbol symbol=PointClass.x source=x type=int32
}

const point = Point { x: 1 };
/// @type.symbol symbol=point type=Point
/// @resolution.name source=Point target=Point

const value: PointClass = point;
/// @type.symbol symbol=value source=value type=PointClass
/// @resolution.name source=PointClass target=PointClass
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Point' is not assignable to type 'PointClass'"
/// @diagnostic.label line=11 column=7 source="const value: PointClass = point;"
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

const point: PointClass = new PointClass();
const value: Point = point;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32
}

class PointClass {
/// @type.symbol symbol=PointClass type=PointClass

    x: int32 = 0;
    /// @type.symbol symbol=PointClass.x source=x type=int32
}

const point = new PointClass();
/// @type.symbol symbol=point type=PointClass
/// @resolution.name source=PointClass target=PointClass
/// @resolution.construct source="new PointClass()" parameters=() return=PointClass kind=class target=PointClass

const value: Point = point;
/// @type.symbol symbol=value source=value type=Point
/// @resolution.name source=Point target=Point
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'PointClass' is not assignable to type 'Point'"
/// @diagnostic.label line=11 column=7 source="const value: Point = point;"
"#,
    );
}
