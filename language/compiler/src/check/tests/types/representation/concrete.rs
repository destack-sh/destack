use crate::tests::{DirRows, TestSession};

#[test]
fn test_concrete_bound_enables_layout_query() {
    let session = TestSession::single(
        r#"
function storageSize<T: Concrete>(): usize {
    const size = comptime sizeOf<T>();
    return size;
}

const size = storageSize<int32>();
size satisfies usize;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
function storageSize<T: Concrete>(): usize {
    const size: usize = comptime sizeOf<T>();
    return size;
}

const size: usize = storageSize<int32>();
size satisfies usize;

=== checked ===
function storageSize<T: Concrete>(): usize {
/// @generic.template symbol=storageSize parameters=(T: Concrete)
/// @type.symbol symbol=storageSize type=<T: Concrete>() => usize
/// @type.symbol symbol=storageSize.T source="T: Concrete" type=T
/// @resolution.name source=Concrete target=memory.capability.Concrete

    const size = comptime sizeOf<T>();
    /// @type.symbol symbol=storageSize.size source=size type=usize
    /// @resolution.name source=sizeOf target=reflect.type.sizeOf
    /// @resolution.call source=sizeOf<T>() parameters=() return=usize kind=symbol target=reflect.type.sizeOf instance=sizeOf<T>
    /// @generic.instance source=sizeOf<T>() id=sizeOf<T>
    /// @resolution.name source=T target=storageSize.T

    return size;
    /// @resolution.name source=size target=storageSize.size

}

const size = storageSize<int32>();
/// @type.symbol symbol=size source=size type=usize
/// @resolution.name source=storageSize target=storageSize
/// @resolution.call source=storageSize<int32>() parameters=() return=usize kind=symbol target=storageSize instance=storageSize<int32>
/// @generic.instance source=storageSize<int32>() id=storageSize<int32>

size satisfies usize;
/// @resolution.name source=size target=size

/// @generic.instance id=sizeOf<T> template=reflect.type.sizeOf arguments=(T)
/// @generic.instance id=storageSize<int32> template=storageSize arguments=(int32)
"#,
    );
}

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

const size = comptime sizeOf<Shape>();
size satisfies usize;
"#,
    );

    session.assert_dir_checked(
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

const size: usize = comptime sizeOf<Shape>();
size satisfies usize;

=== checked ===
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
/// @definition.newtype symbol=Shape source="newtype Shape = Circle | Rectangle" value=Circle | Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

const size = comptime sizeOf<Shape>();
/// @type.symbol symbol=size source=size type=usize
/// @resolution.name source=sizeOf target=reflect.type.sizeOf
/// @resolution.call source=sizeOf<Shape>() parameters=() return=usize kind=symbol target=reflect.type.sizeOf instance=sizeOf<Shape>
/// @generic.instance source=sizeOf<Shape>() id=sizeOf<Shape>
/// @resolution.name source=Shape target=Shape

size satisfies usize;
/// @resolution.name source=size target=size

/// @generic.instance id=sizeOf<Shape> template=reflect.type.sizeOf arguments=(Shape)
"#,
    );
}

#[test]
fn test_dynamic_wrapper_has_concrete_layout() {
    let session = TestSession::single(
        r#"
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

const size = comptime sizeOf<Dynamic<Writer>>();
size satisfies usize;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

const size: usize = comptime sizeOf<Dynamic<Writer>>();
size satisfies usize;

=== checked ===
interface Writer {
/// @type.symbol symbol=Writer type=Writer
/// @definition.interface symbol=Writer
/// @definition.method symbol=Writer.write source="write(bytes: readonly uint8[]): uint" slot=write type=(this: this, readonly Array<uint8>) => uint64

    write(bytes: readonly uint8[]): uint;
    /// @type.symbol symbol=Writer.write source="write(bytes: readonly uint8[]): uint" type=(this: this, readonly Array<uint8>) => uint64
    /// @type.symbol symbol=Writer.write.bytes source="bytes: readonly uint8[]" type=readonly Array<uint8>

}

const size = comptime sizeOf<Dynamic<Writer>>();
/// @type.symbol symbol=size source=size type=usize
/// @resolution.name source=sizeOf target=reflect.type.sizeOf
/// @resolution.call source=sizeOf<Dynamic<Writer>>() parameters=() return=usize kind=symbol target=reflect.type.sizeOf instance=sizeOf<Dynamic<Writer>>
/// @generic.instance source=sizeOf<Dynamic<Writer>>() id=sizeOf<Dynamic<Writer>>
/// @resolution.name source=Dynamic target=memory.dynamic.Dynamic
/// @resolution.name source=Writer target=Writer

size satisfies usize;
/// @resolution.name source=size target=size

/// @generic.instance id=sizeOf<Dynamic<Writer>> template=reflect.type.sizeOf arguments=(Dynamic<Writer>)
"#,
    );
}

#[test]
fn test_dynamic_wrapper_accepts_anonymous_constraint() {
    let session = TestSession::single(
        r#"
type Writer = {
    write(bytes: readonly uint8[]): uint;
};

const size = comptime sizeOf<Dynamic<Writer>>();
size satisfies usize;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
type Writer = {
    write(bytes: readonly uint8[]): uint;
};

const size: usize = comptime sizeOf<Dynamic<Writer>>();
size satisfies usize;

=== checked ===
type Writer = {
/// @type.symbol symbol=Writer type={ write(readonly Array<uint8>): uint64 }
/// @definition.type symbol=Writer value={ write(readonly Array<uint8>): uint64 }

    write(bytes: readonly uint8[]): uint;
    /// @type.symbol symbol=Writer.write.bytes source="bytes: readonly uint8[]" type=readonly Array<uint8>

};

const size = comptime sizeOf<Dynamic<Writer>>();
/// @type.symbol symbol=size source=size type=usize
/// @resolution.name source=sizeOf target=reflect.type.sizeOf
/// @resolution.call source=sizeOf<Dynamic<Writer>>() parameters=() return=usize kind=symbol target=reflect.type.sizeOf instance=sizeOf<Dynamic<Writer>>
/// @generic.instance source=sizeOf<Dynamic<Writer>>() id=sizeOf<Dynamic<Writer>>
/// @resolution.name source=Dynamic target=memory.dynamic.Dynamic
/// @resolution.name source=Writer target=Writer

size satisfies usize;
/// @resolution.name source=size target=size

/// @generic.instance id=sizeOf<Dynamic<Writer>> template=reflect.type.sizeOf arguments=(Dynamic<Writer>)
"#,
    );
}

#[test]
fn test_dynamic_wrapper_requires_dynamic_safe_constraint() {
    let session = TestSession::single(
        r#"
declare const value: Dynamic<<T>(input: T) => T>;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const value: Dynamic<<T>(input: T) => T>;

=== checked ===
declare const value: Dynamic<<T>(input: T) => T>;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.name source=Dynamic target=memory.dynamic.Dynamic
/// @generic.template source=type_expression parameters=(T)
/// @type.symbol symbol=T source=T type=T
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T
"#,
        r#"
/// @diagnostic.error code=EC201 message="type '<T>(T) => T' does not satisfy 'DynamicSafe'"
/// @diagnostic.label line=2 column=30 span="<T>(input: T) => T" line_source="declare const value: Dynamic<<T>(input: T) => T>;"
/// @diagnostic.related file="dynamic.ds" message="required by this bound on 'T'"
"#,
    );
}

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

    session.assert_dir_checked(
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

=== checked ===
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
/// @type.node source="makeCircle() satisfies Shape" type=Shape reduced=Circle | Rectangle
/// @type.node source=makeCircle type=() => Shape
/// @type.node source=makeCircle() type=Shape reduced=Circle | Rectangle
/// @resolution.name source=makeCircle target=makeCircle
/// @resolution.call source=makeCircle() parameters=() return=Shape kind=symbol target=makeCircle
/// @resolution.name source=Shape target=Shape
"#,
    );
}

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
        return Shape(Circle { radius: 1.0 });
    }

    return Shape(Rectangle { width: 1.0, height: 1.0 });
}

makeShape(true) satisfies Shape;
"#,
    );

    session.assert_dir_checked(
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

=== checked ===
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
/// @definition.newtype symbol=Shape source="newtype Shape = Circle | Rectangle" value=Circle | Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

function makeShape(flag: boolean): Shape {
/// @type.symbol symbol=makeShape type=(boolean) => Shape
/// @type.symbol symbol=makeShape.flag source="flag: boolean" type=boolean
/// @resolution.name source=Shape target=Shape

    if (flag) {
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=makeShape.flag

        return Shape(Circle { radius: 1.0 });
        /// @type.node source="Shape(Circle { radius: 1.0 })" type=Shape
        /// @type.node source=Shape type=Shape
        /// @resolution.name source=Shape target=Shape
        /// @resolution.construct source="Shape(Circle { radius: 1.0 })" parameters=(Circle | Rectangle) arguments=(provided(Circle { radius: 1.0 }) as Circle | Rectangle) return=Shape kind=newtype target=Shape
        /// @type.node source="Circle { radius: 1.0 }" type=Circle
        /// @resolution.name source=Circle target=Circle
        /// @type.node source=1.0 type=1

    }

    return Shape(Rectangle { width: 1.0, height: 1.0 });
    /// @type.node source="Shape(Rectangle { width: 1.0, height: 1.0 })" type=Shape
    /// @type.node source=Shape type=Shape
    /// @resolution.name source=Shape target=Shape
    /// @resolution.construct source="Shape(Rectangle { width: 1.0, height: 1.0 })" parameters=(Circle | Rectangle) arguments=(provided(Rectangle { width: 1.0, height: 1.0 }) as Circle | Rectangle) return=Shape kind=newtype target=Shape
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

    session.assert_dir_checked(
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

=== checked ===
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
/// @type.node source="makeShape(true) satisfies Shape" type=Shape reduced=Circle | Rectangle
/// @type.node source=makeShape type=(boolean) => Shape
/// @type.node source=makeShape(true) type=Shape reduced=Circle | Rectangle
/// @resolution.name source=makeShape target=makeShape
/// @resolution.call source=makeShape(true) parameters=(boolean) arguments=(provided(true) as boolean) return=Shape kind=symbol target=makeShape
/// @type.node source=true type=true
/// @resolution.name source=Shape target=Shape
"#,
    );
}
