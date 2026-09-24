use crate::tests::{DirRows, TestSession};

/// A Concrete bound allows a layout query on the bounded parameter.
#[test]
fn test_concrete_bound_enables_layout_query() {
    let session = TestSession::single(
        r#"
function storageSize<T: Concrete>(): usize {
    const size = const sizeOf<T>();
    return size;
}

const size = storageSize<int32>();
size satisfies usize;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
function storageSize<T: Concrete>(): usize {
    const size: usize = const sizeOf<T>();
    return size;
}

const size: usize = storageSize<int32>();
size satisfies usize;

=== dir ===
function storageSize<T: Concrete>(): usize {
/// @generic.template symbol=storageSize parameters=(T: Concrete)
/// @type.symbol symbol=storageSize type=<T: Concrete>() => usize
/// @type.symbol symbol=storageSize.T source="T: Concrete" type=T
/// @resolution.name source=Concrete target=Concrete

    const size = const sizeOf<T>();
    /// @type.symbol symbol=storageSize.size source=size type=usize
    /// @resolution.pattern source=size kind=binding target=storageSize.size
    /// @resolution.name source=sizeOf target=sizeOf
    /// @resolution.call source=sizeOf<T>() parameters=() return=usize kind=symbol target=sizeOf instance=sizeOf<T>
    /// @generic.instantiation id=sizeOf<T> template=sizeOf arguments=(T) owner=storageSize
    /// @generic.instance id=sizeOf<T> template=sizeOf arguments=(T)
    /// @resolution.name source=T target=storageSize.T

    return size;
    /// @resolution.name source=size target=storageSize.size
    /// @resolution.place source=size placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=size root=storageSize.size

}

const size = storageSize<int32>();
/// @type.symbol symbol=size source=size type=usize
/// @resolution.pattern source=size kind=binding target=size
/// @resolution.name source=storageSize target=storageSize
/// @resolution.call source=storageSize<int32>() parameters=() return=usize kind=symbol target=storageSize instance=storageSize<int32>
/// @generic.instantiation id=storageSize<int32> template=storageSize arguments=(int32)
/// @generic.instance id=sizeOf<int32> template=sizeOf arguments=(int32)
/// @generic.instance id=storageSize<int32> template=storageSize arguments=(int32)

size satisfies usize;
/// @resolution.name source=size target=size
/// @resolution.place source=size placement="local" lifetime="static" access="immutable"
/// @resolution.access source=size root=size
"#,
    );
}

/// A newtype over a union of structs has a concrete layout.
#[test]
fn test_newtype_union_has_concrete_layout() {
    let session = TestSession::single(
        r#"
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

newtype Shape = Circle | Rectangle;

const size = const sizeOf<Shape>();
size satisfies usize;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

newtype Shape = Circle | Rectangle;

const size: usize = const sizeOf<Shape>();
size satisfies usize;

=== dir ===
struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.field symbol=Circle.radius source="radius: float64" key=radius type=float64

    radius: float64;
    /// @type.symbol symbol=Circle.radius source="radius: float64" type=float64

}

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.height source="height: float64" key=height type=float64
/// @definition.field symbol=Rectangle.width source="width: float64" key=width type=float64

    width: float64;
    /// @type.symbol symbol=Rectangle.width source="width: float64" type=float64

    height: float64;
    /// @type.symbol symbol=Rectangle.height source="height: float64" type=float64

}

newtype Shape = Circle | Rectangle;
/// @type.symbol symbol=Shape source="newtype Shape = Circle | Rectangle" type=Shape
/// @definition.newtype symbol=Shape source="newtype Shape = Circle | Rectangle" backing=Circle | Rectangle constructors=[(Circle) => Shape, (Rectangle) => Shape, (Circle | Rectangle) => Shape]
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

const size = const sizeOf<Shape>();
/// @type.symbol symbol=size source=size type=usize
/// @resolution.pattern source=size kind=binding target=size
/// @resolution.name source=sizeOf target=sizeOf
/// @resolution.call source=sizeOf<Shape>() parameters=() return=usize kind=symbol target=sizeOf instance=sizeOf<Shape>
/// @generic.instantiation id=sizeOf<Shape> template=sizeOf arguments=(Shape)
/// @generic.instance id=sizeOf<Shape> template=sizeOf arguments=(Shape)
/// @resolution.name source=Shape target=Shape

size satisfies usize;
/// @resolution.name source=size target=size
/// @resolution.place source=size placement="local" lifetime="static" access="immutable"
/// @resolution.access source=size root=size
"#,
    );
}

/// A Dynamic over an interface has a concrete layout.
#[test]
fn test_dynamic_has_concrete_layout() {
    let session = TestSession::single(
        r#"
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

const size = const sizeOf<Dynamic<Writer>>();
size satisfies usize;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

const size: usize = const sizeOf<Dynamic<Writer>>();
size satisfies usize;

=== dir ===
interface Writer {
/// @generic.template symbol=Writer parameters=(this: Writer)
/// @type.symbol symbol=Writer type=Writer
/// @definition.interface symbol=Writer template=(this: Writer)
/// @definition.where symbol=Writer relation=satisfies left=this right=Writer
/// @definition.method symbol=Writer.write source="write(bytes: readonly uint8[]): uint" slot=write type=(readonly uint8[]) => uint64

    write(bytes: readonly uint8[]): uint;
    /// @type.symbol symbol=Writer.write source="write(bytes: readonly uint8[]): uint" type=(readonly uint8[]) => uint64
    /// @generic.instance id=Array<uint8> template=Array arguments=(uint8)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<uint8>> template=sliceAssumeInit arguments=(MaybeUninit<uint8>)
    /// @generic.instance id=sliceUninit<MaybeUninit<uint8>> template=sliceUninit arguments=(MaybeUninit<uint8>)
    /// @type.symbol symbol=Writer.write.bytes source="bytes: readonly uint8[]" type=readonly uint8[]

}

const size = const sizeOf<Dynamic<Writer>>();
/// @type.symbol symbol=size source=size type=usize
/// @resolution.pattern source=size kind=binding target=size
/// @resolution.name source=sizeOf target=sizeOf
/// @resolution.call source=sizeOf<Dynamic<Writer>>() parameters=() return=usize kind=symbol target=sizeOf instance=sizeOf<Dynamic<Writer>>
/// @generic.instantiation id=sizeOf<Dynamic<Writer>> template=sizeOf arguments=(Dynamic<Writer>)
/// @generic.instance id=sizeOf<Dynamic<Writer>> template=sizeOf arguments=(Dynamic<Writer>)
/// @resolution.name source=Dynamic target=Dynamic
/// @resolution.name source=Writer target=Writer

size satisfies usize;
/// @resolution.name source=size target=size
/// @resolution.place source=size placement="local" lifetime="static" access="immutable"
/// @resolution.access source=size root=size
"#,
    );
}

/// A Dynamic accepts an anonymous object type as its constraint.
#[test]
fn test_dynamic_accepts_anonymous_constraint() {
    let session = TestSession::single(
        r#"
type Writer = {
    write(bytes: readonly uint8[]): uint;
};

const size = const sizeOf<Dynamic<Writer>>();
size satisfies usize;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
type Writer = {
    write(bytes: readonly uint8[]): uint;
};

const size: usize = const sizeOf<Dynamic<Writer>>();
size satisfies usize;

=== dir ===
type Writer = {
/// @type.symbol symbol=Writer type={ write(readonly uint8[]): uint64 }
/// @generic.instance id=Array<uint8> template=Array arguments=(uint8)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<uint8>> template=sliceAssumeInit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceUninit<MaybeUninit<uint8>> template=sliceUninit arguments=(MaybeUninit<uint8>)
/// @definition.type symbol=Writer value={ write(readonly uint8[]): uint64 }

    write(bytes: readonly uint8[]): uint;
};

const size = const sizeOf<Dynamic<Writer>>();
/// @type.symbol symbol=size source=size type=usize
/// @resolution.pattern source=size kind=binding target=size
/// @resolution.name source=sizeOf target=sizeOf
/// @resolution.call source=sizeOf<Dynamic<Writer>>() parameters=() return=usize kind=symbol target=sizeOf instance=sizeOf<Dynamic<Writer>>
/// @generic.instantiation id=sizeOf<Dynamic<Writer>> template=sizeOf arguments=(Dynamic<Writer>)
/// @generic.instance id=sizeOf<Dynamic<Writer>> template=sizeOf arguments=(Dynamic<Writer>)
/// @resolution.name source=Dynamic target=Dynamic
/// @resolution.name source=Writer target=Writer

size satisfies usize;
/// @resolution.name source=size target=size
/// @resolution.place source=size placement="local" lifetime="static" access="immutable"
/// @resolution.access source=size root=size
"#,
    );
}

/// A Dynamic over a generic signature reports a diagnostic.
#[test]
fn test_dynamic_requires_dynamic_safe_constraint() {
    let session = TestSession::single(
        r#"
declare const value: Dynamic<<T>(input: T) => T>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const value: Dynamic<<T>(input: T) => T>;

=== dir ===
declare const value: Dynamic<<T>(input: T) => T>;
/// @type.symbol symbol=value source=value type=Dynamic<<T>(T) => T>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Dynamic target=Dynamic
/// @generic.template source=type_expression parameters=(T)
/// @type.symbol symbol=T source=T type=T
/// @type.symbol symbol=input source="input: T" type=T
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T
"#,
        r#"
/// @diagnostic.error id=not-erasable message="type '<T>(input: T) => T' cannot be erased into 'DynamicSafe'"
/// @diagnostic.label line=2 column=30 span="<T>(input: T) => T" line_source="declare const value: Dynamic<<T>(input: T) => T>;"
/// @diagnostic.related file="dynamic.ds" line=7 column=21 span="T" line_source="export type Dynamic<T: DynamicSafe> = intrinsic;" message="required by this bound on 'T'"
/// @diagnostic.help message="prove the source erasable with a DynamicSafe bound"
"#,
    );
}

/// A function returning a union alias keeps the declared alias.
#[test]
fn test_alias_return_preserves_declared_union_type() {
    let session = TestSession::single(
        r#"
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

function makeCircle(): Shape {
    return Circle { radius: 1.0 };
}

makeCircle() satisfies Shape;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

function makeCircle(): Shape {
    return Circle { radius: 1.0 } as Shape;
}

makeCircle() satisfies Shape;

=== dir ===
struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.field symbol=Circle.radius source="radius: float64" key=radius type=float64

    radius: float64;
    /// @type.symbol symbol=Circle.radius source="radius: float64" type=float64

}

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.height source="height: float64" key=height type=float64
/// @definition.field symbol=Rectangle.width source="width: float64" key=width type=float64

    width: float64;
    /// @type.symbol symbol=Rectangle.width source="width: float64" type=float64

    height: float64;
    /// @type.symbol symbol=Rectangle.height source="height: float64" type=float64

}

type Shape = Circle | Rectangle;
/// @type.symbol symbol=Shape source="type Shape = Circle | Rectangle" type=Circle | Rectangle
/// @definition.type symbol=Shape source="type Shape = Circle | Rectangle" value=Circle | Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

function makeCircle(): Shape {
/// @type.symbol symbol=makeCircle type=() => Shape
/// @resolution.name source=Shape target=Shape

    return Circle { radius: 1.0 };
    /// @type.node source="Circle { radius: 1.0 }" type=Circle
    /// @resolution.name source=Circle target=Circle
    /// @type.node source=1.0 type=1

}

makeCircle() satisfies Shape;
/// @type.node source="makeCircle() satisfies Shape" type=Shape
/// @type.node source=makeCircle type=() => Shape
/// @type.node source=makeCircle() type=Shape
/// @resolution.name source=makeCircle target=makeCircle
/// @resolution.call source=makeCircle() parameters=() return=Shape kind=symbol target=makeCircle
/// @resolution.name source=Shape target=Shape
"#,
    );
}

/// A function returning a newtype over a union constructs it from either variant.
#[test]
fn test_newtype_return_allows_multiple_variants() {
    let session = TestSession::single(
        r#"
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

newtype Shape = Circle | Rectangle;

function makeShape(flag: boolean): Shape {
    if (flag) {
        return Shape(Circle { radius: 1.0 } as Circle | Rectangle);
    }

    return Shape(Rectangle { width: 1.0, height: 1.0 } as Circle | Rectangle);
}

makeShape(true) satisfies Shape;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

newtype Shape = Circle | Rectangle;

function makeShape(flag: boolean): Shape {
    if (flag) {
        return Shape(Circle { radius: 1.0 } as Circle | Rectangle);
    }

    return Shape(Rectangle { width: 1.0, height: 1.0 } as Circle | Rectangle);
}

makeShape(true) satisfies Shape;

=== dir ===
struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.field symbol=Circle.radius source="radius: float64" key=radius type=float64

    radius: float64;
    /// @type.symbol symbol=Circle.radius source="radius: float64" type=float64

}

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.height source="height: float64" key=height type=float64
/// @definition.field symbol=Rectangle.width source="width: float64" key=width type=float64

    width: float64;
    /// @type.symbol symbol=Rectangle.width source="width: float64" type=float64

    height: float64;
    /// @type.symbol symbol=Rectangle.height source="height: float64" type=float64

}

newtype Shape = Circle | Rectangle;
/// @type.symbol symbol=Shape source="newtype Shape = Circle | Rectangle" type=Shape
/// @definition.newtype symbol=Shape source="newtype Shape = Circle | Rectangle" backing=Circle | Rectangle constructors=[(Circle) => Shape, (Rectangle) => Shape, (Circle | Rectangle) => Shape]
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

function makeShape(flag: boolean): Shape {
/// @type.symbol symbol=makeShape type=(boolean) => Shape
/// @type.symbol symbol=makeShape.flag source="flag: boolean" type=boolean
/// @resolution.name source=Shape target=Shape

    if (flag) {
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=makeShape.flag
    /// @resolution.place source=flag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=flag root=makeShape.flag

        return Shape(Circle { radius: 1.0 } as Circle | Rectangle);
        /// @type.node source="Shape(Circle { radius: 1.0 } as Circle | Rectangle)" type=Shape
        /// @type.node source=Shape type=Shape
        /// @resolution.name source=Shape target=Shape
        /// @resolution.construct source="Shape(Circle { radius: 1.0 } as Circle | Rectangle)" parameters=(Circle | Rectangle) arguments=(provided(Circle { radius: 1.0 } as Circle | Rectangle) as Circle | Rectangle) return=Shape kind=newtype target=Shape backing=Circle | Rectangle
        /// @type.node source="Circle { radius: 1.0 } as Circle | Rectangle" type=Circle | Rectangle
        /// @type.node source="Circle { radius: 1.0 }" type=Circle
        /// @resolution.name source=Circle target=Circle
        /// @type.node source=1.0 type=1
        /// @resolution.name source=Circle target=Circle
        /// @resolution.name source=Rectangle target=Rectangle

    }

    return Shape(Rectangle { width: 1.0, height: 1.0 } as Circle | Rectangle);
    /// @type.node source="Shape(Rectangle { width: 1.0, height: 1.0 } as Circle | Rectangle)" type=Shape
    /// @type.node source=Shape type=Shape
    /// @resolution.name source=Shape target=Shape
    /// @resolution.construct source="Shape(Rectangle { width: 1.0, height: 1.0 } as Circle | Rectangle)" parameters=(Circle | Rectangle) arguments=(provided(Rectangle { width: 1.0, height: 1.0 } as Circle | Rectangle) as Circle | Rectangle) return=Shape kind=newtype target=Shape backing=Circle | Rectangle
    /// @type.node source="Rectangle { width: 1.0, height: 1.0 } as Circle | Rectangle" type=Circle | Rectangle
    /// @type.node source="Rectangle { width: 1.0, height: 1.0 }" type=Rectangle
    /// @resolution.name source=Rectangle target=Rectangle
    /// @type.node source=1.0 type=1
    /// @type.node source=1.0 type=1
    /// @resolution.name source=Circle target=Circle
    /// @resolution.name source=Rectangle target=Rectangle

}

makeShape(true) satisfies Shape;
/// @type.node source="makeShape(true) satisfies Shape" type=Shape
/// @type.node source=makeShape type=(boolean) => Shape
/// @type.node source=makeShape(true) type=Shape
/// @resolution.name source=makeShape target=makeShape
/// @resolution.call source=makeShape(true) parameters=(boolean) arguments=(provided(true) as boolean) return=Shape kind=symbol target=makeShape
/// @type.node source=true type=true
/// @resolution.name source=Shape target=Shape
"#,
    );
}

/// A function returning a union alias accepts a value of each member.
#[test]
fn test_alias_return_accepts_each_union_member() {
    let session = TestSession::single(
        r#"
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

function makeShape(flag: boolean): Shape {
    if (flag) {
        return Circle { radius: 1.0 };
    }

    return Rectangle { width: 1.0, height: 1.0 };
}

makeShape(true) satisfies Shape;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

function makeShape(flag: boolean): Shape {
    if (flag) {
        return Circle { radius: 1.0 } as Shape;
    }

    return Rectangle { width: 1.0, height: 1.0 } as Shape;
}

makeShape(true) satisfies Shape;

=== dir ===
struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.field symbol=Circle.radius source="radius: float64" key=radius type=float64

    radius: float64;
    /// @type.symbol symbol=Circle.radius source="radius: float64" type=float64

}

struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.field symbol=Rectangle.height source="height: float64" key=height type=float64
/// @definition.field symbol=Rectangle.width source="width: float64" key=width type=float64

    width: float64;
    /// @type.symbol symbol=Rectangle.width source="width: float64" type=float64

    height: float64;
    /// @type.symbol symbol=Rectangle.height source="height: float64" type=float64

}

type Shape = Circle | Rectangle;
/// @type.symbol symbol=Shape source="type Shape = Circle | Rectangle" type=Circle | Rectangle
/// @definition.type symbol=Shape source="type Shape = Circle | Rectangle" value=Circle | Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

function makeShape(flag: boolean): Shape {
/// @type.symbol symbol=makeShape type=(boolean) => Shape
/// @type.symbol symbol=makeShape.flag source="flag: boolean" type=boolean
/// @resolution.name source=Shape target=Shape

    if (flag) {
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=makeShape.flag
    /// @resolution.place source=flag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=flag root=makeShape.flag

        return Circle { radius: 1.0 };
        /// @type.node source="Circle { radius: 1.0 }" type=Circle
        /// @resolution.name source=Circle target=Circle
        /// @type.node source=1.0 type=1

    }

    return Rectangle { width: 1.0, height: 1.0 };
    /// @type.node source="Rectangle { width: 1.0, height: 1.0 }" type=Rectangle
    /// @resolution.name source=Rectangle target=Rectangle
    /// @type.node source=1.0 type=1
    /// @type.node source=1.0 type=1

}

makeShape(true) satisfies Shape;
/// @type.node source="makeShape(true) satisfies Shape" type=Shape
/// @type.node source=makeShape type=(boolean) => Shape
/// @type.node source=makeShape(true) type=Shape
/// @resolution.name source=makeShape target=makeShape
/// @resolution.call source=makeShape(true) parameters=(boolean) arguments=(provided(true) as boolean) return=Shape kind=symbol target=makeShape
/// @type.node source=true type=true
/// @resolution.name source=Shape target=Shape
"#,
    );
}
