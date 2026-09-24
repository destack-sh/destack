use crate::tests::{DirRows, TestSession};

/// A struct satisfies a structural interface matching its fields.
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_coercion(),
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

=== dir ===
interface HasX {
/// @generic.template symbol=HasX parameters=(this: HasX)
/// @type.symbol symbol=HasX type=HasX
/// @definition.interface symbol=HasX template=(this: HasX)
/// @definition.where symbol=HasX relation=satisfies left=this right=HasX
/// @definition.field symbol=HasX.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=HasX.x source="x: int32" type=int32

}

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const value: HasX = Point { x: 1 };
/// @type.symbol symbol=value source=value type=HasX
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=HasX target=HasX
/// @resolution.name source=Point target=Point
/// @coercion.node source="Point { x: 1 }" from=Point adjustments=[{ kind: erase, target: HasX }] origin=implicit
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int32 }] origin=implicit

value satisfies HasX;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.name source=HasX target=HasX
"#,
    );
}

/// An interface inheriting a nominal interface requires a written implements clause.
#[test]
fn test_struct_requires_nominal_inherited_interface() {
    let session = TestSession::single(
        r#"
newtype interface Named {
    name: string;
}

interface Drawable extends Named {
    opacity: float32;
}

struct Picture {
    name: string;
    opacity: float32;
}

declare const picture: Picture;
picture satisfies Drawable;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Named {
    name: string;
}

interface Drawable extends Named {
    opacity: float32;
}

struct Picture {
    name: string;
    opacity: float32;
}

declare const picture: Picture;
picture satisfies Drawable;

=== dir ===
newtype interface Named {
/// @generic.template symbol=Named parameters=(this: Named)
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named template=(this: Named) nominal=true
/// @definition.where symbol=Named relation=satisfies left=this right=Named
/// @definition.field symbol=Named.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Named.name source="name: string" type=string

}

interface Drawable extends Named {
/// @generic.template symbol=Drawable parameters=(this: Drawable)
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable template=(this: Drawable)
/// @definition.where symbol=Drawable relation=satisfies left=this right=Drawable
/// @definition.extends symbol=Drawable source=Named target=Named
/// @definition.field symbol=Drawable.opacity source="opacity: float32" key=opacity type=float32
/// @resolution.name source=Named target=Named

    opacity: float32;
    /// @type.symbol symbol=Drawable.opacity source="opacity: float32" type=float32

}

struct Picture {
/// @type.symbol symbol=Picture type=Picture
/// @definition.struct symbol=Picture
/// @definition.field symbol=Picture.name source="name: string" key=name type=string
/// @definition.field symbol=Picture.opacity source="opacity: float32" key=opacity type=float32

    name: string;
    /// @type.symbol symbol=Picture.name source="name: string" type=string

    opacity: float32;
    /// @type.symbol symbol=Picture.opacity source="opacity: float32" type=float32

}

declare const picture: Picture;
/// @type.symbol symbol=picture source=picture type=Picture
/// @resolution.pattern source=picture kind=binding target=picture
/// @resolution.name source=Picture target=Picture

picture satisfies Drawable;
/// @resolution.name source=picture target=picture
/// @resolution.place source=picture placement="local" lifetime="static" access="immutable"
/// @resolution.access source=picture root=picture
/// @resolution.name source=Drawable target=Drawable
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Picture' does not satisfy 'Drawable'"
/// @diagnostic.label line=16 column=1 span="picture" line_source="picture satisfies Drawable;"
"#,
    );
}

/// A struct implementing the nominal parent satisfies the structural child.
#[test]
fn test_struct_satisfies_structural_interface_with_nominal_heritage() {
    let session = TestSession::single(
        r#"
newtype interface Named {
    name: string;
}

interface Drawable extends Named {
    opacity: float32;
}

struct Picture implements Named {
    name: string;
    opacity: float32;
}

declare const picture: Picture;
picture satisfies Drawable;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Named {
    name: string;
}

interface Drawable extends Named {
    opacity: float32;
}

struct Picture implements Named {
    name: string;
    opacity: float32;
}

declare const picture: Picture;
picture satisfies Drawable;

=== dir ===
newtype interface Named {
/// @generic.template symbol=Named parameters=(this: Named)
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named template=(this: Named) nominal=true
/// @definition.where symbol=Named relation=satisfies left=this right=Named
/// @definition.field symbol=Named.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Named.name source="name: string" type=string

}

interface Drawable extends Named {
/// @generic.template symbol=Drawable parameters=(this: Drawable)
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable template=(this: Drawable)
/// @definition.where symbol=Drawable relation=satisfies left=this right=Drawable
/// @definition.extends symbol=Drawable source=Named target=Named
/// @definition.field symbol=Drawable.opacity source="opacity: float32" key=opacity type=float32
/// @resolution.name source=Named target=Named

    opacity: float32;
    /// @type.symbol symbol=Drawable.opacity source="opacity: float32" type=float32

}

struct Picture implements Named {
/// @type.symbol symbol=Picture type=Picture
/// @definition.struct symbol=Picture
/// @definition.where symbol=Picture source=Named relation=satisfies left=this right=Named
/// @definition.implements symbol=Picture source=Named target=Named
/// @definition.field symbol=Picture.name source="name: string" key=name type=string
/// @definition.field symbol=Picture.opacity source="opacity: float32" key=opacity type=float32
/// @definition.conformance symbol=Picture member=Picture.name requirement=Named.name
/// @resolution.name source=Named target=Named

    name: string;
    /// @type.symbol symbol=Picture.name source="name: string" type=string

    opacity: float32;
    /// @type.symbol symbol=Picture.opacity source="opacity: float32" type=float32

}

declare const picture: Picture;
/// @type.symbol symbol=picture source=picture type=Picture
/// @resolution.pattern source=picture kind=binding target=picture
/// @resolution.name source=Picture target=Picture

picture satisfies Drawable;
/// @resolution.name source=picture target=picture
/// @resolution.place source=picture placement="local" lifetime="static" access="immutable"
/// @resolution.access source=picture root=picture
/// @resolution.name source=Drawable target=Drawable
"#,
    );
}

/// A struct satisfies an object type without assigning to one.
#[test]
fn test_struct_satisfies_but_does_not_store_as_object_type() {
    // keep structural width for a satisfies check, object-typed storage stays exact
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

const value: { readonly x: int32 } = point;
/// @type.symbol symbol=value source=value type={ readonly x: int32 }
/// @resolution.pattern source=value kind=binding target=value
/// @type.symbol symbol=x#1 source="readonly x: int32" type=int32
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point

value satisfies { readonly x: int32 };
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @type.symbol symbol=x#2 source="readonly x: int32" type=int32
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Point' is not assignable to type '{ readonly x: int32 }'"
/// @diagnostic.label line=7 column=38 span="point" line_source="const value: { readonly x: int32 } = point;"
/// @diagnostic.related line=7 column=14 span="{ readonly x: int32 }" line_source="const value: { readonly x: int32 } = point;" message="expected due to this annotation"
/// @diagnostic.note message="'{ readonly x: int32 }' stores its exact object type, declare an interface to accept structurally wider values"
"#,
    );
}

/// A struct satisfies an interface declaring the field as optional.
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

    session.assert_dir(
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

=== dir ===
interface HasCount {
/// @generic.template symbol=HasCount parameters=(this: HasCount)
/// @type.symbol symbol=HasCount type=HasCount
/// @definition.interface symbol=HasCount template=(this: HasCount)
/// @definition.where symbol=HasCount relation=satisfies left=this right=HasCount
/// @definition.field symbol=HasCount.count source="count?: int32" key=count type=int32

    count?: int32;
    /// @type.symbol symbol=HasCount.count source="count?: int32" type=int32

}

struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32" key=count type=int32

    count: int32;
    /// @type.symbol symbol=Counter.count source="count: int32" type=int32

}

const counter: HasCount = Counter { count: 1 };
/// @type.symbol symbol=counter source=counter type=HasCount
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=HasCount target=HasCount
/// @resolution.name source=Counter target=Counter

counter satisfies HasCount;
/// @resolution.name source=counter target=counter
/// @resolution.place source=counter placement="local" lifetime="static" access="immutable"
/// @resolution.access source=counter root=counter
/// @resolution.name source=HasCount target=HasCount
"#,
    );
}

/// An implements clause without the declared members reports a diagnostic.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
interface Drawable {
/// @generic.template symbol=Drawable parameters=(this: Drawable)
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable template=(this: Drawable)
/// @definition.where symbol=Drawable relation=satisfies left=this right=Drawable
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=() => void

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=() => void

}

struct Point implements Drawable {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.where symbol=Point source=Drawable relation=satisfies left=this right=Drawable
/// @definition.implements symbol=Point source=Drawable target=Drawable
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @resolution.name source=Drawable target=Drawable

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'Point' does not implement interface 'Drawable'"
/// @diagnostic.label line=6 column=25 span="Drawable" line_source="struct Point implements Drawable {"
"#,
    );
}

/// Implementing two interfaces disagreeing on one generic parent reports a diagnostic.
#[test]
fn test_struct_implements_rejects_conflicting_generic_heritage() {
    let session = TestSession::single(
        r#"
interface Base<in out T> {}
interface Left extends Base<string> {}
interface Right extends Base<int32> {}

struct Point implements Left, Right {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Base<in out T> {}
interface Left extends Base<string> {}
interface Right extends Base<int32> {}

struct Point implements Left, Right {}

=== dir ===
interface Base<in out T> {}
/// @generic.template symbol=Base parameters=(in out T, this: Base<T>)
/// @type.symbol symbol=Base source="interface Base<in out T> {}" type=Base
/// @definition.interface symbol=Base source="interface Base<in out T> {}" template=(in out T, this: Base<T>)
/// @definition.where symbol=Base source="interface Base<in out T> {}" relation=satisfies left=this right=Base<T>
/// @type.symbol symbol=Base.T source="in out T" type=T

interface Left extends Base<string> {}
/// @generic.template symbol=Left parameters=(this: Left)
/// @type.symbol symbol=Left source="interface Left extends Base<string> {}" type=Left
/// @definition.interface symbol=Left source="interface Left extends Base<string> {}" template=(this: Left)
/// @definition.where symbol=Left source="interface Left extends Base<string> {}" relation=satisfies left=this right=Left
/// @definition.extends symbol=Left source=Base<string> target=Base<string>
/// @resolution.name source=Base target=Base

interface Right extends Base<int32> {}
/// @generic.template symbol=Right parameters=(this: Right)
/// @type.symbol symbol=Right source="interface Right extends Base<int32> {}" type=Right
/// @definition.interface symbol=Right source="interface Right extends Base<int32> {}" template=(this: Right)
/// @definition.where symbol=Right source="interface Right extends Base<int32> {}" relation=satisfies left=this right=Right
/// @definition.extends symbol=Right source=Base<int32> target=Base<int32>
/// @resolution.name source=Base target=Base

struct Point implements Left, Right {}
/// @type.symbol symbol=Point source="struct Point implements Left, Right {}" type=Point
/// @definition.struct symbol=Point source="struct Point implements Left, Right {}"
/// @definition.where symbol=Point source=Left relation=satisfies left=this right=Left
/// @definition.implements symbol=Point source=Left target=Left
/// @definition.where symbol=Point source=Right relation=satisfies left=this right=Right
/// @definition.implements symbol=Point source=Right target=Right
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right
"#,
        r#"
/// @diagnostic.error id=conflicting-heritage message="type 'Point' has conflicting heritage for 'Base'"
/// @diagnostic.label line=6 column=31 span="Right" line_source="struct Point implements Left, Right {}"
"#,
    );
}

/// Assigning a struct to a class of the same shape reports a diagnostic.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

class PointClass {
/// @type.symbol symbol=PointClass type=typeof PointClass
/// @definition.class symbol=PointClass
/// @definition.field symbol=PointClass.x source="x: int32 = 0" key=x type=int32

    x: int32 = 0;
    /// @type.symbol symbol=PointClass.x source="x: int32 = 0" type=int32

}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

const value: PointClass = point;
/// @type.symbol symbol=value source=value type=PointClass
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=PointClass target=PointClass
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Point' is not assignable to type 'PointClass'"
/// @diagnostic.label line=11 column=27 span="point" line_source="const value: PointClass = point;"
/// @diagnostic.related line=11 column=14 span="PointClass" line_source="const value: PointClass = point;" message="expected due to this annotation"
"#,
    );
}

/// Assigning a class to a struct of the same shape reports a diagnostic.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

class PointClass {
/// @type.symbol symbol=PointClass type=typeof PointClass
/// @definition.class symbol=PointClass
/// @definition.field symbol=PointClass.x source="x: int32 = 0" key=x type=int32

    x: int32 = 0;
    /// @type.symbol symbol=PointClass.x source="x: int32 = 0" type=int32

}

const point = new PointClass();
/// @type.symbol symbol=point source=point type=PointClass
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.construct source="new PointClass()" parameters=() return=PointClass kind=class target=PointClass constructor=default
/// @resolution.name source=PointClass target=PointClass

const value: Point = point;
/// @type.symbol symbol=value source=value type=Point
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Point target=Point
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'PointClass' is not assignable to type 'Point'"
/// @diagnostic.label line=11 column=22 span="point" line_source="const value: Point = point;"
/// @diagnostic.related line=11 column=14 span="Point" line_source="const value: Point = point;" message="expected due to this annotation"
"#,
    );
}
