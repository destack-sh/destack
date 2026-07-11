use crate::tests::{DirRows, TestSession};

#[test]
fn test_mutable_array_rejects_covariant_element_flow() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: Shape[] = circles;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: Shape[] = circles;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const circles: Circle[];
/// @type.symbol symbol=circles source=circles type=Array<Circle>
/// @resolution.name source=Circle target=Circle

const shapes: Shape[] = circles;
/// @type.symbol symbol=shapes source=shapes type=Array<Shape>
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Array<Circle>' is not assignable to type 'Array<Shape>'"
/// @diagnostic.label line=6 column=25 span="circles" line_source="const shapes: Shape[] = circles;"
/// @diagnostic.note message="the mismatch is in the element type: expected 'Shape', found 'Circle'"
"#,
    );
}

#[test]
fn test_readonly_array_accepts_covariant_element_flow() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: readonly Shape[] = circles;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: readonly Shape[] = circles;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const circles: Circle[];
/// @type.symbol symbol=circles source=circles type=Array<Circle>
/// @resolution.name source=Circle target=Circle

const shapes: readonly Shape[] = circles;
/// @type.symbol symbol=shapes source=shapes type=readonly Array<Shape>
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles
"#,
    );
}

#[test]
fn test_readonly_array_rejects_element_injection() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}
class Square extends Shape {}

declare const circles: Circle[];
const widened: readonly (Circle | Square)[] = circles;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}
class Square extends Shape {}

declare const circles: Circle[];
const widened: readonly (Circle | Square)[] = circles;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Square extends Shape {}
/// @type.symbol symbol=Square source="class Square extends Shape {}" type=Square
/// @definition.class symbol=Square source="class Square extends Shape {}"
/// @definition.extends symbol=Square source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const circles: Circle[];
/// @type.symbol symbol=circles source=circles type=Array<Circle>
/// @resolution.name source=Circle target=Circle

const widened: readonly (Circle | Square)[] = circles;
/// @type.symbol symbol=widened source=widened type=readonly Array<Circle | Square>
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=circles target=circles
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Array<Circle>' is not assignable to type 'readonly Array<Circle | Square>'"
/// @diagnostic.label line=7 column=47 span="circles" line_source="const widened: readonly (Circle | Square)[] = circles;"
"#,
    );
}

#[test]
fn test_function_interior_conversions_do_not_flow() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}
class Square extends Shape {}

declare const make: () => Circle;
const widened: () => Shape = make;
const either: () => Circle | Square = make;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}
class Square extends Shape {}

declare const make: () => Circle;
const widened: () => Shape = make;
const either: () => Circle | Square = make;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Square extends Shape {}
/// @type.symbol symbol=Square source="class Square extends Shape {}" type=Square
/// @definition.class symbol=Square source="class Square extends Shape {}"
/// @definition.extends symbol=Square source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const make: () => Circle;
/// @type.symbol symbol=make source=make type=Function<(), Circle>
/// @resolution.name source=Circle target=Circle

const widened: () => Shape = make;
/// @type.symbol symbol=widened source=widened type=Function<(), Shape>
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=make target=make

const either: () => Circle | Square = make;
/// @type.symbol symbol=either source=either type=Function<(), Circle | Square>
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=make target=make
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '() => Circle' is not assignable to type '() => Circle | Square'"
/// @diagnostic.label line=8 column=39 span="make" line_source="const either: () => Circle | Square = make;"
"#,
    );
}

#[test]
fn test_declared_covariance_rejects_invariant_use() {
    let session = TestSession::single(
        r#"
declare class Evil<out T> {
    slot: T;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare class Evil<out T> {
    slot: T;
}

=== checked ===
declare class Evil<out T> {
/// @generic.template symbol=Evil parameters=(out T)
/// @type.symbol symbol=Evil type=Evil
/// @definition.class symbol=Evil template=(out T)
/// @definition.field symbol=Evil.slot source="slot: T" key=slot type=T
/// @type.symbol symbol=Evil.T source="out T" type=T

    slot: T;
    /// @type.symbol symbol=Evil.slot source="slot: T" type=T
    /// @resolution.name source=T target=Evil.T

}
"#,
        r#"
/// @diagnostic.error code=EC443 message="generic parameter 'T' is used invariantly and cannot be declared 'out'"
/// @diagnostic.label line=2 column=24 span="T" line_source="declare class Evil<out T> {"
"#,
    );
}

#[test]
fn test_declared_covariance_rejects_contravariant_use() {
    let session = TestSession::single(
        r#"
struct Sink<out T> {
    readonly accept: (value: T) => void;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Sink<out T> {
    readonly accept: (arg0: T) => void;
}

=== checked ===
struct Sink<out T> {
/// @generic.template symbol=Sink parameters=(out T)
/// @type.symbol symbol=Sink type=Sink
/// @definition.struct symbol=Sink template=(out T)
/// @definition.field symbol=Sink.accept source="readonly accept: (value: T) => void" key=accept type=Function<(T,), void>
/// @type.symbol symbol=Sink.T source="out T" type=T

    readonly accept: (value: T) => void;
    /// @type.symbol symbol=Sink.accept source="readonly accept: (value: T) => void" type=Function<(T,), void>
    /// @resolution.name source=T target=Sink.T

}
"#,
        r#"
/// @diagnostic.error code=EC443 message="generic parameter 'T' is used contravariantly and cannot be declared 'out'"
/// @diagnostic.label line=2 column=17 span="T" line_source="struct Sink<out T> {"
"#,
    );
}

#[test]
fn test_declared_covariance_binds_the_value_context() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

struct Box<out T> {
    value: T;
}

declare const owned: Box<Circle>;
const copy: Box<Shape> = owned;

declare const aliased: Managed<Box<Circle>>;
const widened: Managed<Box<Shape>> = aliased;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

struct Box<out T> {
    value: T;
}

declare const owned: Box<Circle>;
const copy: Box<Shape> = owned;

declare const aliased: Managed<Box<Circle>>;
const widened: Managed<Box<Shape>> = aliased;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

struct Box<out T> {
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @type.symbol symbol=Box.T source="out T" type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

declare const owned: Box<Circle>;
/// @type.symbol symbol=owned source=owned type=Box<Circle>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Circle target=Circle

const copy: Box<Shape> = owned;
/// @type.symbol symbol=copy source=copy type=Box<Shape>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=owned target=owned

declare const aliased: Managed<Box<Circle>>;
/// @type.symbol symbol=aliased source=aliased type=Managed<Box<Circle>>
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Box target=Box
/// @resolution.name source=Circle target=Circle

const widened: Managed<Box<Shape>> = aliased;
/// @type.symbol symbol=widened source=widened type=Managed<Box<Shape>>
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Box target=Box
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=aliased target=aliased

/// @generic.instance id=Box<Circle> template=Box arguments=(Circle)
/// @generic.instance id=Box<Shape> template=Box arguments=(Shape)
/// @generic.instance id=Managed<Box<Circle>> template=memory.managed.Managed arguments=(Box<Circle>)
/// @generic.instance id=Managed<Box<Shape>> template=memory.managed.Managed arguments=(Box<Shape>)
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Managed<Box<Circle>>' is not assignable to type 'Managed<Box<Shape>>'"
/// @diagnostic.label line=13 column=38 span="aliased" line_source="const widened: Managed<Box<Shape>> = aliased;"
/// @diagnostic.note message="'Managed<Box<Circle>>' reduces to 'Box<Circle>'"
/// @diagnostic.note message="'Managed<Box<Shape>>' reduces to 'Box<Shape>'"
"#,
    );
}

#[test]
fn test_declared_covariance_accepts_readonly_surfaces() {
    let session = TestSession::single(
        r#"
declare class Reader<out T> {
    readonly value: T;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare class Reader<out T> {
    readonly value: T;
}

=== checked ===
declare class Reader<out T> {
/// @generic.template symbol=Reader parameters=(out T)
/// @type.symbol symbol=Reader type=Reader
/// @definition.class symbol=Reader template=(out T)
/// @definition.field symbol=Reader.value source="readonly value: T" key=value type=T
/// @type.symbol symbol=Reader.T source="out T" type=T

    readonly value: T;
    /// @type.symbol symbol=Reader.value source="readonly value: T" type=T
    /// @resolution.name source=T target=Reader.T

}
"#,
    );
}

#[test]
fn test_mutable_object_rejects_covariant_property_flow() {
    let session = TestSession::single(
        r#"
declare const point: { x: 1 };
const widened: { x: float64 } = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const point: { x: 1 };
const widened: { x: float64 } = point;

=== checked ===
declare const point: { x: 1 };
/// @type.symbol symbol=point source=point type={ x: 1 }

const widened: { x: float64 } = point;
/// @type.symbol symbol=widened source=widened type={ x: float64 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '{ x: 1 }' is not assignable to type '{ x: float64 }'"
/// @diagnostic.label line=3 column=33 span="point" line_source="const widened: { x: float64 } = point;"
/// @diagnostic.note message="the mismatch is in field 'x': expected 'float64', found '1'"
"#,
    );
}

#[test]
fn test_readonly_object_field_widens_identity_edges_only() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare const point: { x: Circle };
const widened: { readonly x: Shape } = point;

declare const scalar: { x: 1 };
const converted: { readonly x: float64 } = scalar;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare const point: { x: Circle };
const widened: { readonly x: Shape } = point;

declare const scalar: { x: 1 };
const converted: { readonly x: float64 } = scalar;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const point: { x: Circle };
/// @type.symbol symbol=point source=point type={ x: Circle }
/// @resolution.name source=Circle target=Circle

const widened: { readonly x: Shape } = point;
/// @type.symbol symbol=widened source=widened type={ readonly x: Shape }
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=point target=point

declare const scalar: { x: 1 };
/// @type.symbol symbol=scalar source=scalar type={ x: 1 }

const converted: { readonly x: float64 } = scalar;
/// @type.symbol symbol=converted source=converted type={ readonly x: float64 }
/// @resolution.name source=scalar target=scalar
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '{ x: 1 }' is not assignable to type '{ x: float64 }'"
/// @diagnostic.label line=9 column=44 span="scalar" line_source="const converted: { readonly x: float64 } = scalar;"
/// @diagnostic.note message="the mismatch is in field 'x': expected 'float64', found '1'"
"#,
    );
}

#[test]
fn test_function_parameters_flow_contravariantly() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare const useShape: (shape: Shape) => void;
const useCircle: (circle: Circle) => void = useShape;

declare const useCircle2: (circle: Circle) => void;
const useShape2: (shape: Shape) => void = useCircle2;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare const useShape: (arg0: Shape) => void;
const useCircle: (arg0: Circle) => void = useShape;

declare const useCircle2: (arg0: Circle) => void;
const useShape2: (arg0: Shape) => void = useCircle2;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const useShape: (shape: Shape) => void;
/// @type.symbol symbol=useShape source=useShape type=Function<(Shape,), void>
/// @resolution.name source=Shape target=Shape

const useCircle: (circle: Circle) => void = useShape;
/// @type.symbol symbol=useCircle source=useCircle type=Function<(Circle,), void>
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=useShape target=useShape

declare const useCircle2: (circle: Circle) => void;
/// @type.symbol symbol=useCircle2 source=useCircle2 type=Function<(Circle,), void>
/// @resolution.name source=Circle target=Circle

const useShape2: (shape: Shape) => void = useCircle2;
/// @type.symbol symbol=useShape2 source=useShape2 type=Function<(Shape,), void>
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=useCircle2 target=useCircle2
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '(Circle) => void' is not assignable to type '(Shape) => void'"
/// @diagnostic.label line=9 column=43 span="useCircle2" line_source="const useShape2: (shape: Shape) => void = useCircle2;"
"#,
    );
}

#[test]
fn test_intrinsic_newtype_arguments_stay_invariant() {
    let session = TestSession::single(
        r#"
newtype Handle<T> = intrinsic;

declare const source: Handle<int32>;
const target: Handle<string> = source;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Handle<in out T> = intrinsic;

declare const source: Handle<int32>;
const target: Handle<string> = source;

=== checked ===
newtype Handle<T> = intrinsic;
/// @generic.template symbol=Handle parameters=(in out T)
/// @type.symbol symbol=Handle source="newtype Handle<T> = intrinsic" type=Handle
/// @definition.newtype symbol=Handle source="newtype Handle<T> = intrinsic" template=(in out T) value=intrinsic
/// @type.symbol symbol=Handle.T source=T type=T

declare const source: Handle<int32>;
/// @type.symbol symbol=source source=source type=Handle<int32>
/// @resolution.name source=Handle target=Handle

const target: Handle<string> = source;
/// @type.symbol symbol=target source=target type=Handle<string>
/// @resolution.name source=Handle target=Handle
/// @type.node source=source type=Handle<int32>
/// @resolution.name source=source target=source
/// @generic.instance source=source id=Handle<int32>

/// @generic.instance id=Handle<int32> template=Handle arguments=(int32)
/// @generic.instance id=Handle<string> template=Handle arguments=(string)
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Handle<int32>' is not assignable to type 'Handle<string>'"
/// @diagnostic.label line=5 column=32 span="source" line_source="const target: Handle<string> = source;"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Handle': expected 'string', found 'int32'"
"#,
    );
}

#[test]
fn test_intrinsic_newtype_infers_from_the_expected_instantiation() {
    let session = TestSession::single(
        r#"
newtype Handle<T> = intrinsic;

extension<T> of Handle<[T]> {
    @intrinsic("memory.unique.empty")
    static empty(): Handle<[T]>;
}

class Holder {
    storage: Handle<[uint8]> = Handle.empty();
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Handle<in out T> = intrinsic;

extension<T> of Handle<[T]> {
    @intrinsic("memory.unique.empty")
    static empty(): Handle<[T]>;
}

class Holder {
    storage: Handle<[uint8]> = Handle.empty<uint8>();
}

=== checked ===
newtype Handle<T> = intrinsic;
/// @generic.template symbol=Handle parameters=(in out T#1)
/// @type.symbol symbol=Handle source="newtype Handle<T> = intrinsic" type=Handle
/// @definition.newtype symbol=Handle source="newtype Handle<T> = intrinsic" template=(in out T#1) value=intrinsic
/// @type.symbol symbol=Handle.T source=T type=T#1

extension<T> of Handle<[T]> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Handle<Slice<T#2>>
/// @definition.method symbol=empty source="static empty(): Handle<[T]>" slot=empty static=true type=() => Handle<Slice<T#2>>
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=T target=T

    @intrinsic("memory.unique.empty")
    /// @resolution.name source=intrinsic target=decorator.intrinsic.intrinsic

    static empty(): Handle<[T]>;
    /// @type.symbol symbol=empty source="static empty(): Handle<[T]>" type=() => Handle<Slice<T#2>>
    /// @resolution.name source=Handle target=Handle
    /// @resolution.name source=T target=T

}

class Holder {
/// @type.symbol symbol=Holder type=Holder
/// @definition.class symbol=Holder
/// @definition.field symbol=Holder.storage source="storage: Handle<[uint8]> = Handle.empty()" key=storage type=Handle<Slice<uint8>>

    storage: Handle<[uint8]> = Handle.empty();
    /// @type.symbol symbol=Holder.storage source="storage: Handle<[uint8]> = Handle.empty()" type=Handle<Slice<uint8>>
    /// @resolution.name source=Handle target=Handle
    /// @type.node source=Handle type=Handle
    /// @type.node source=Handle.empty type=() => Handle<Slice<T#2>>
    /// @type.node source=Handle.empty() type=Handle<Slice<uint8>>
    /// @resolution.name source=Handle target=Handle
    /// @resolution.member source=Handle.empty receiver=Handle kind=symbol target=empty
    /// @resolution.call source=Handle.empty() parameters=() return=Handle<Slice<uint8>> kind=symbol target=empty receiver=Handle instance=Handle<Slice<T#2>>.<extension#1>.empty
    /// @generic.instance source=Handle.empty id=Handle<Slice<T#2>>
    /// @generic.instance source=Handle.empty() id=Handle<Slice<T#2>>.<extension#1>.empty
    /// @generic.instance source=Handle.empty() id=Handle<Slice<uint8>>

}

/// @generic.instance id=Handle<Slice<T#2>> template=Handle arguments=(Slice<T#2>)
/// @generic.instance id=Handle<Slice<T#2>>.<extension#1>.empty template=empty arguments=(uint8)
/// @generic.instance id=Handle<Slice<uint8>> template=Handle arguments=(Slice<uint8>)
"#,
        r#"
"#,
    );
}
