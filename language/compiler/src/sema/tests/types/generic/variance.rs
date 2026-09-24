use crate::tests::{DirRows, TestSession};

/// A mutable array rejects an assignment that widens its element type.
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: Shape[] = circles;

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const circles: Circle[];
/// @type.symbol symbol=circles source=circles type=Circle[]
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Circle target=Circle

const shapes: Shape[] = circles;
/// @type.symbol symbol=shapes source=shapes type=Shape[]
/// @resolution.pattern source=shapes kind=binding target=shapes
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Circle[]' is not assignable to type 'Shape[]'"
/// @diagnostic.label line=6 column=25 span="circles" line_source="const shapes: Shape[] = circles;"
/// @diagnostic.related line=6 column=15 span="Shape[]" line_source="const shapes: Shape[] = circles;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Array': expected 'Shape', found 'Circle'"
"#,
    );
}

/// A readonly array accepts an assignment that widens its element type.
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: readonly Shape[] = circles;

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const circles: Circle[];
/// @type.symbol symbol=circles source=circles type=Circle[]
/// @resolution.pattern source=circles kind=binding target=circles
/// @generic.instance id=Array<Circle> template=Array arguments=(Circle)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<Circle>> template=sliceAssumeInit arguments=(MaybeUninit<Circle>)
/// @generic.instance id=sliceUninit<MaybeUninit<Circle>> template=sliceUninit arguments=(MaybeUninit<Circle>)
/// @resolution.name source=Circle target=Circle

const shapes: readonly Shape[] = circles;
/// @type.symbol symbol=shapes source=shapes type=readonly Shape[]
/// @resolution.pattern source=shapes kind=binding target=shapes
/// @generic.instance id=Array<Shape> template=Array arguments=(Shape)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<Shape>> template=sliceAssumeInit arguments=(MaybeUninit<Shape>)
/// @generic.instance id=sliceUninit<MaybeUninit<Shape>> template=sliceUninit arguments=(MaybeUninit<Shape>)
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
    );
}

/// A readonly array rejects an element type widened into a union.
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}
class Square extends Shape {}

declare const circles: Circle[];
const widened: readonly (Circle | Square)[] = circles;

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Square extends Shape {}
/// @type.symbol symbol=Square source="class Square extends Shape {}" type=typeof Square
/// @definition.class symbol=Square source="class Square extends Shape {}"
/// @definition.extends symbol=Square source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const circles: Circle[];
/// @type.symbol symbol=circles source=circles type=Circle[]
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Circle target=Circle

const widened: readonly (Circle | Square)[] = circles;
/// @type.symbol symbol=widened source=widened type=readonly Circle | Square[]
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=circles target=circles
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Circle[]' is not assignable to type 'readonly Circle | Square[]'"
/// @diagnostic.label line=7 column=47 span="circles" line_source="const widened: readonly (Circle | Square)[] = circles;"
/// @diagnostic.related line=7 column=16 span="readonly" line_source="const widened: readonly (Circle | Square)[] = circles;" message="expected due to this annotation"
"#,
    );
}

/// A function type keeps its result exact under assignment.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Square extends Shape {}
/// @type.symbol symbol=Square source="class Square extends Shape {}" type=typeof Square
/// @definition.class symbol=Square source="class Square extends Shape {}"
/// @definition.extends symbol=Square source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const make: () => Circle;
/// @type.symbol symbol=make source=make type=() => Circle
/// @resolution.pattern source=make kind=binding target=make
/// @resolution.name source=Circle target=Circle

const widened: () => Shape = make;
/// @type.symbol symbol=widened source=widened type=() => Shape
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=make target=make
/// @resolution.place source=make placement="local" lifetime="static" access="immutable"
/// @resolution.access source=make root=make

const either: () => Circle | Square = make;
/// @type.symbol symbol=either source=either type=() => Circle | Square
/// @resolution.pattern source=either kind=binding target=either
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=make target=make
/// @resolution.place source=make placement="local" lifetime="static" access="immutable"
/// @resolution.access source=make root=make
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '() => Circle' is not assignable to type '() => Circle | Square'"
/// @diagnostic.label line=8 column=39 span="make" line_source="const either: () => Circle | Square = make;"
/// @diagnostic.related line=8 column=15 span="() => Circle | Square" line_source="const either: () => Circle | Square = make;" message="expected due to this annotation"
"#,
    );
}

/// A parameter declared covariant reports a diagnostic at a mutable field.
#[test]
fn test_declared_covariance_rejects_invariant_use() {
    let session = TestSession::single(
        r#"
declare class Evil<out T> {
    slot: T;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare class Evil<out T> {
    slot: T;
}

=== dir ===
declare class Evil<out T> {
/// @generic.template symbol=Evil parameters=(out T)
/// @type.symbol symbol=Evil type=typeof Evil
/// @definition.class symbol=Evil template=(out T)
/// @definition.field symbol=Evil.slot source="slot: T" key=slot type=T
/// @type.symbol symbol=Evil.T source="out T" type=T

    slot: T;
    /// @type.symbol symbol=Evil.slot source="slot: T" type=T
    /// @resolution.name source=T target=Evil.T

}
"#,
        r#"
/// @diagnostic.error id=variance-conflict message="generic parameter 'T' is used invariantly and cannot be declared 'out'"
/// @diagnostic.label line=2 column=24 span="T" line_source="declare class Evil<out T> {"
"#,
    );
}

/// A parameter declared covariant reports a diagnostic at a callback parameter.
#[test]
fn test_declared_covariance_rejects_contravariant_use() {
    let session = TestSession::single(
        r#"
struct Sink<out T> {
    readonly accept: (value: T) => void;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Sink<out T> {
    readonly accept: (value: T) => void;
}

=== dir ===
struct Sink<out T> {
/// @generic.template symbol=Sink parameters=(out T)
/// @type.symbol symbol=Sink type=Sink
/// @definition.struct symbol=Sink template=(out T)
/// @definition.field symbol=Sink.accept source="readonly accept: (value: T) => void" key=accept type=(T) => void
/// @type.symbol symbol=Sink.T source="out T" type=T

    readonly accept: (value: T) => void;
    /// @type.symbol symbol=Sink.accept source="readonly accept: (value: T) => void" type=(T) => void
    /// @type.symbol symbol=Sink.value source="value: T" type=T
    /// @resolution.name source=T target=Sink.T

}
"#,
        r#"
/// @diagnostic.error id=variance-conflict message="generic parameter 'T' is used contravariantly and cannot be declared 'out'"
/// @diagnostic.label line=2 column=17 span="T" line_source="struct Sink<out T> {"
"#,
    );
}

/// A struct with a covariant parameter widens as a value.
#[test]
fn test_widen_a_covariant_struct_as_a_value() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

struct Box<out T> {
    value: T;
}

declare const owned: Box<Circle>;
const copy: Box<Shape> = owned;
"#,
    );

    session.assert_dir_and_diagnostics(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
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
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=Box target=Box
/// @resolution.name source=Circle target=Circle

const copy: Box<Shape> = owned;
/// @type.symbol symbol=copy source=copy type=Box<Shape>
/// @resolution.pattern source=copy kind=binding target=copy
/// @resolution.name source=Box target=Box
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=owned target=owned
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
"#,
        r#"

"#,
    );
}

/// A parameter declared covariant accepts a readonly field.
#[test]
fn test_declared_covariance_accepts_readonly_surfaces() {
    let session = TestSession::single(
        r#"
declare class Reader<out T> {
    readonly value: T;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare class Reader<out T> {
    readonly value: T;
}

=== dir ===
declare class Reader<out T> {
/// @generic.template symbol=Reader parameters=(out T)
/// @type.symbol symbol=Reader type=typeof Reader
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

/// A mutable object type rejects an assignment that widens a property.
#[test]
fn test_mutable_object_rejects_covariant_property_flow() {
    let session = TestSession::single(
        r#"
declare const point: { x: 1 };
const widened: { x: float64 } = point;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const point: { x: 1 };
const widened: { x: float64 } = point;

=== dir ===
declare const point: { x: 1 };
/// @type.symbol symbol=point source=point type={ x: 1 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x#1 source="x: 1" type=1

const widened: { x: float64 } = point;
/// @type.symbol symbol=widened source=widened type={ x: float64 }
/// @resolution.pattern source=widened kind=binding target=widened
/// @type.symbol symbol=x#2 source="x: float64" type=float64
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ x: 1 }' is not assignable to type '{ x: float64 }'"
/// @diagnostic.label line=3 column=33 span="point" line_source="const widened: { x: float64 } = point;"
/// @diagnostic.related line=3 column=16 span="{ x: float64 }" line_source="const widened: { x: float64 } = point;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'x': expected 'float64', found '1'"
"#,
    );
}

/// An aliased object value rejects widening one of its fields.
#[test]
fn test_object_storage_rejects_aliased_field_widening() {
    // keep aliased object values exact, widening a field requires an interface
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const point: { x: Circle };
/// @type.symbol symbol=point source=point type={ x: Circle }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x#1 source="x: Circle" type=Circle
/// @resolution.name source=Circle target=Circle

const widened: { readonly x: Shape } = point;
/// @type.symbol symbol=widened source=widened type={ readonly x: Shape }
/// @resolution.pattern source=widened kind=binding target=widened
/// @type.symbol symbol=x#2 source="readonly x: Shape" type=Shape
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point

declare const scalar: { x: 1 };
/// @type.symbol symbol=scalar source=scalar type={ x: 1 }
/// @resolution.pattern source=scalar kind=binding target=scalar
/// @type.symbol symbol=x#3 source="x: 1" type=1

const converted: { readonly x: float64 } = scalar;
/// @type.symbol symbol=converted source=converted type={ readonly x: float64 }
/// @resolution.pattern source=converted kind=binding target=converted
/// @type.symbol symbol=x#4 source="readonly x: float64" type=float64
/// @resolution.name source=scalar target=scalar
/// @resolution.place source=scalar placement="local" lifetime="static" access="immutable"
/// @resolution.access source=scalar root=scalar
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ x: Circle }' is not assignable to type '{ readonly x: Shape }'"
/// @diagnostic.label line=6 column=40 span="point" line_source="const widened: { readonly x: Shape } = point;"
/// @diagnostic.related line=6 column=16 span="{ readonly x: Shape }" line_source="const widened: { readonly x: Shape } = point;" message="expected due to this annotation"
/// @diagnostic.note message="'{ readonly x: Shape }' stores its exact object type, declare an interface to accept structurally wider values"
/// @diagnostic.error id=not-assignable message="type '{ x: 1 }' is not assignable to type '{ readonly x: float64 }'"
/// @diagnostic.label line=9 column=44 span="scalar" line_source="const converted: { readonly x: float64 } = scalar;"
/// @diagnostic.related line=9 column=18 span="{ readonly x: float64 }" line_source="const converted: { readonly x: float64 } = scalar;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'x': expected 'float64', found '1'"
"#,
    );
}

/// A function type accepts a wider parameter and rejects a narrower one.
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare const useShape: (shape: Shape) => void;
const useCircle: (circle: Circle) => void = useShape;

declare const useCircle2: (circle: Circle) => void;
const useShape2: (shape: Shape) => void = useCircle2;

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const useShape: (shape: Shape) => void;
/// @type.symbol symbol=useShape source=useShape type=(Shape) => void
/// @resolution.pattern source=useShape kind=binding target=useShape
/// @type.symbol symbol=shape#1 source="shape: Shape" type=Shape
/// @resolution.name source=Shape target=Shape

const useCircle: (circle: Circle) => void = useShape;
/// @type.symbol symbol=useCircle source=useCircle type=(Circle) => void
/// @resolution.pattern source=useCircle kind=binding target=useCircle
/// @type.symbol symbol=circle#1 source="circle: Circle" type=Circle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=useShape target=useShape
/// @resolution.place source=useShape placement="local" lifetime="static" access="immutable"
/// @resolution.access source=useShape root=useShape

declare const useCircle2: (circle: Circle) => void;
/// @type.symbol symbol=useCircle2 source=useCircle2 type=(Circle) => void
/// @resolution.pattern source=useCircle2 kind=binding target=useCircle2
/// @type.symbol symbol=circle#2 source="circle: Circle" type=Circle
/// @resolution.name source=Circle target=Circle

const useShape2: (shape: Shape) => void = useCircle2;
/// @type.symbol symbol=useShape2 source=useShape2 type=(Shape) => void
/// @resolution.pattern source=useShape2 kind=binding target=useShape2
/// @type.symbol symbol=shape#2 source="shape: Shape" type=Shape
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=useCircle2 target=useCircle2
/// @resolution.place source=useCircle2 placement="local" lifetime="static" access="immutable"
/// @resolution.access source=useCircle2 root=useCircle2
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '(circle: Circle) => void' is not assignable to type '(shape: Shape) => void'"
/// @diagnostic.label line=9 column=43 span="useCircle2" line_source="const useShape2: (shape: Shape) => void = useCircle2;"
/// @diagnostic.related line=9 column=18 span="(shape: Shape) => void" line_source="const useShape2: (shape: Shape) => void = useCircle2;" message="expected due to this annotation"
"#,
    );
}

/// An intrinsic newtype keeps its type arguments invariant.
#[test]
fn test_intrinsic_newtype_arguments_stay_invariant() {
    let session = TestSession::single(
        r#"
newtype Handle<T> = intrinsic;

declare const source: Handle<int32>;
const target: Handle<string> = source;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Handle<in out T> = intrinsic;

declare const source: Handle<int32>;
const target: Handle<string> = source;

=== dir ===
newtype Handle<T> = intrinsic;
/// @generic.template symbol=Handle parameters=(in out T)
/// @type.symbol symbol=Handle source="newtype Handle<T> = intrinsic" type=Handle
/// @definition.newtype symbol=Handle source="newtype Handle<T> = intrinsic" template=(in out T) backing=intrinsic constructors=[<T>(intrinsic) => Handle<T>]
/// @type.symbol symbol=Handle.T source=T type=T

declare const source: Handle<int32>;
/// @type.symbol symbol=source source=source type=Handle<int32>
/// @resolution.pattern source=source kind=binding target=source
/// @resolution.name source=Handle target=Handle

const target: Handle<string> = source;
/// @type.symbol symbol=target source=target type=Handle<string>
/// @resolution.pattern source=target kind=binding target=target
/// @resolution.name source=Handle target=Handle
/// @type.node source=source type=Handle<int32>
/// @resolution.name source=source target=source
/// @resolution.place source=source placement="local" lifetime="static" access="immutable"
/// @resolution.access source=source root=source
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Handle<int32>' is not assignable to type 'Handle<string>'"
/// @diagnostic.label line=5 column=32 span="source" line_source="const target: Handle<string> = source;"
/// @diagnostic.related line=5 column=15 span="Handle" line_source="const target: Handle<string> = source;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Handle': expected 'string', found 'int32'"
"#,
    );
}

/// An intrinsic newtype infers its arguments from the expected instantiation.
#[test]
fn test_intrinsic_newtype_infers_from_the_expected_instantiation() {
    let session = TestSession::single(
        r#"
newtype Handle<T> = intrinsic;

@intrinsic("memory.unique.empty")
declare function emptyHandle<T>(): Handle<[T]>;

extension<T> of Handle<[T]> {
    static empty(): Handle<[T]> {
        return emptyHandle<T>();
    }
}

class Holder {
    storage: Handle<[uint8]> = Handle.empty();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Handle<in out T> = intrinsic;

@intrinsic("memory.unique.empty")
declare function emptyHandle<T>(): Handle<[T]>;

extension<T> of Handle<[T]> {
    static empty(): Handle<[T]> {
        return emptyHandle<T>();
    }
}

class Holder {
    storage: Handle<[uint8]> = Handle.empty<uint8>();
}

=== dir ===
newtype Handle<T> = intrinsic;
/// @generic.template symbol=Handle parameters=(in out T#1)
/// @type.symbol symbol=Handle source="newtype Handle<T> = intrinsic" type=Handle
/// @definition.newtype symbol=Handle source="newtype Handle<T> = intrinsic" template=(in out T#1) backing=intrinsic constructors=[<T#1>(intrinsic) => Handle<T#1>]
/// @type.symbol symbol=Handle.T source=T type=T#1

@intrinsic("memory.unique.empty")
/// @type.node source=intrinsic type=intrinsic
/// @resolution.name source=intrinsic target=intrinsic
/// @type.node source="\"memory.unique.empty\"" type="memory.unique.empty"

declare function emptyHandle<T>(): Handle<[T]>;
/// @generic.template symbol=emptyHandle parameters=(T#2)
/// @type.symbol symbol=emptyHandle source="declare function emptyHandle<T>(): Handle<[T]>" type=<T#2>() => Handle<Slice<T#2>>
/// @type.symbol symbol=emptyHandle.T source=T type=T#2
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=T target=emptyHandle.T

extension<T> of Handle<[T]> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Handle<Slice<T#3>>
/// @definition.method symbol=empty slot=empty static=true type=() => Handle<Slice<T#3>>
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=T target=T

    static empty(): Handle<[T]> {
    /// @type.symbol symbol=empty type=() => Handle<Slice<T#3>>
    /// @resolution.name source=Handle target=Handle
    /// @resolution.name source=T target=T

        return emptyHandle<T>();
        /// @type.node source=emptyHandle type=() => Handle<Slice<T#3>>
        /// @type.node source=emptyHandle<T>() type=Handle<Slice<T#3>>
        /// @resolution.name source=emptyHandle target=emptyHandle
        /// @resolution.call source=emptyHandle<T>() parameters=() return=Handle<Slice<T#3>> kind=symbol target=emptyHandle instance=emptyHandle<T#3>
        /// @generic.instantiation id=emptyHandle<T#3> template=emptyHandle arguments=(T#3) owner=empty
        /// @resolution.name source=T target=T

    }
}

class Holder {
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder
/// @definition.field symbol=Holder.storage source="storage: Handle<[uint8]> = Handle.empty()" key=storage type=Handle<Slice<uint8>>

    storage: Handle<[uint8]> = Handle.empty();
    /// @type.symbol symbol=Holder.storage source="storage: Handle<[uint8]> = Handle.empty()" type=Handle<Slice<uint8>>
    /// @resolution.name source=Handle target=Handle
    /// @type.node source=Handle type=Handle
    /// @type.node source=Handle.empty type=() => Handle<Slice<T#3>>
    /// @type.node source=Handle.empty() type=Handle<Slice<uint8>>
    /// @resolution.name source=Handle target=Handle
    /// @resolution.member source=Handle.empty receiver=Handle type=() => Handle<Slice<T#3>> kind=symbol target_receiver=Handle target=empty
    /// @resolution.call source=Handle.empty() parameters=() return=Handle<Slice<uint8>> kind=symbol target=empty instance=Handle<Slice<T#3>>.<extension#1>.empty
    /// @generic.instantiation id=empty<uint8> template=empty arguments=(uint8)

}
"#,
        r#"
"#,
    );
}

/// A callable with a narrower result assigns to a parameter expecting a wider one.
#[test]
fn test_assign_a_callable_with_a_narrower_result_to_a_wider_parameter() {
    let session = TestSession::single(
        r#"
declare function take(callback: (value: int32) => int32 | undefined): void;

function increment(value: int32): int32 {
    return value + 1;
}

take(increment);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(callback: (value: int32) => int32 | undefined): void;

function increment(value: int32): int32 {
    return value + 1;
}

take(increment);

=== dir ===
declare function take(callback: (value: int32) => int32 | undefined): void;
/// @type.symbol symbol=take source="declare function take(callback: (value: int32) => int32 | undefined): void" type=((int32) => int32 | undefined) => void
/// @type.symbol symbol=take.value source="value: int32" type=int32

function increment(value: int32): int32 {
/// @type.symbol symbol=increment type=(int32) => int32
/// @type.symbol symbol=increment.value source="value: int32" type=int32

    return value + 1;
    /// @resolution.name source=value target=increment.value
    /// @resolution.operator source="value + 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=increment.value

}

take(increment);
/// @resolution.name source=take target=take
/// @resolution.call source=take(increment) parameters=((int32) => int32 | undefined) arguments=(provided(increment) as (int32) => int32 | undefined) return=void kind=symbol target=take
/// @resolution.name source=increment target=increment
/// @resolution.function source=increment type=Function<(int32,), int32, "readonly"> target=increment
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'Function<(value: int32,), int32, \"readonly\">' is not assignable to parameter of type '(value: int32) => int32 | undefined'"
/// @diagnostic.label line=8 column=6 span="increment" line_source="take(increment);"
/// @diagnostic.related line=8 column=1 span="take(increment)" line_source="take(increment);" message="in this call"
"#,
    );
}

/// Discard an async callback result against a void-returning parameter.
#[test]
fn test_accept_an_async_callback_at_a_void_returning_parameter() {
    let session = TestSession::single(
        r#"
declare function forEach(visit: (value: int32) => void): void;

forEach(async (value) => value);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function forEach(visit: (value: int32) => void): void;

forEach(async (value: int32) => value);

=== dir ===
declare function forEach(visit: (value: int32) => void): void;
/// @type.symbol symbol=forEach source="declare function forEach(visit: (value: int32) => void): void" type=((int32) => void) => void
/// @type.symbol symbol=forEach.value source="value: int32" type=int32

forEach(async (value) => value);
/// @resolution.name source=forEach target=forEach
/// @resolution.call source="forEach(async (value) => value)" parameters=((int32) => void) arguments=(provided(async (value) => value) as (int32) => void) return=void kind=symbol target=forEach
/// @type.symbol symbol=symbol4 source="async (value) => value" type=Function<(int32,), Promise<int32>, "readonly">
/// @resolution.call source="async (value) => value" parameters=(^Function<(), int32, "once">) arguments=(supplied(0) as ^Function<(), int32, "once">) return=Promise<int32> kind=symbol target=Promise.create instance=Promise.create<int32>
/// @generic.instantiation id=Promise.create<int32> template=Promise.create arguments=(int32)
/// @type.symbol symbol=symbol4.value source=value type=int32
/// @resolution.name source=value target=symbol4.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol4.value
"#,
        r#"
"#,
    );
}

/// An owned struct value widens its type argument covariantly.
#[test]
fn test_owned_struct_value_widens_covariantly() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

struct Box<Value> {
    value: Value;
}

declare const circle: Circle;

const exact: Box<Circle> = Box { value: circle };
const widened: Box<Shape> = exact;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

struct Box<out Value> {
    value: Value;
}

declare const circle: Circle;

const exact: Box<Circle> = Box<Circle> { value: circle };
const widened: Box<Shape> = exact;

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

struct Box<Value> {
/// @generic.template symbol=Box parameters=(out Value)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out Value)
/// @definition.field symbol=Box.value source="value: Value" key=value type=Value
/// @type.symbol symbol=Box.Value source=Value type=Value

    value: Value;
    /// @type.symbol symbol=Box.value source="value: Value" type=Value
    /// @resolution.name source=Value target=Box.Value

}

declare const circle: Circle;
/// @type.symbol symbol=circle source=circle type=Circle
/// @resolution.pattern source=circle kind=binding target=circle
/// @resolution.name source=Circle target=Circle

const exact: Box<Circle> = Box { value: circle };
/// @type.symbol symbol=exact source=exact type=Box<Circle>
/// @resolution.pattern source=exact kind=binding target=exact
/// @generic.instance id=Box<Circle> template=Box arguments=(Circle)
/// @resolution.name source=Box target=Box
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Box target=Box
/// @resolution.name source=circle target=circle
/// @resolution.place source=circle placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circle root=circle

const widened: Box<Shape> = exact;
/// @type.symbol symbol=widened source=widened type=Box<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @generic.instance id=Box<Shape> template=Box arguments=(Shape)
/// @resolution.name source=Box target=Box
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=exact target=exact
/// @resolution.place source=exact placement="local" lifetime="static" access="immutable"
/// @resolution.access source=exact root=exact
"#,
    );
}

/// A readonly borrow widens the type argument of the struct it lends.
#[test]
fn test_readonly_borrow_widens_the_struct_payload() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

struct Box<Value> {
    value: Value;
}

declare const circle: Circle;

const exact: Box<Circle> = Box { value: circle };
const widened: &readonly Box<Shape> = &readonly exact;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

struct Box<out Value> {
    value: Value;
}

declare const circle: Circle;

const exact: Box<Circle> = Box<Circle> { value: circle };
const widened: &'static readonly Box<Shape> = &readonly exact;

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

struct Box<Value> {
/// @generic.template symbol=Box parameters=(out Value)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out Value)
/// @definition.field symbol=Box.value source="value: Value" key=value type=Value
/// @type.symbol symbol=Box.Value source=Value type=Value

    value: Value;
    /// @type.symbol symbol=Box.value source="value: Value" type=Value
    /// @resolution.name source=Value target=Box.Value

}

declare const circle: Circle;
/// @type.symbol symbol=circle source=circle type=Circle
/// @resolution.pattern source=circle kind=binding target=circle
/// @resolution.name source=Circle target=Circle

const exact: Box<Circle> = Box { value: circle };
/// @type.symbol symbol=exact source=exact type=Box<Circle>
/// @resolution.pattern source=exact kind=binding target=exact
/// @generic.instance id=Box<Circle> template=Box arguments=(Circle)
/// @resolution.name source=Box target=Box
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Box target=Box
/// @resolution.name source=circle target=circle
/// @resolution.place source=circle placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circle root=circle

const widened: &readonly Box<Shape> = &readonly exact;
/// @type.symbol symbol=widened source=widened type=&'static readonly Box<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @generic.instance id=Box<Shape> template=Box arguments=(Shape)
/// @resolution.name source=Box target=Box
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=exact target=exact
/// @resolution.place source=exact placement="local" lifetime="static" access="immutable"
/// @resolution.access source=exact root=exact
"#,
    );
}

/// An exclusive borrow rejects widening the type argument of the struct it lends.
#[test]
fn test_reject_struct_payload_widening_under_a_mutable_borrow() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

struct Box<Value> {
    value: Value;
}

declare const circle: Circle;

let exact: Box<Circle> = Box { value: circle };
const widened: &Box<Shape> = &exact;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

struct Box<out Value> {
    value: Value;
}

declare const circle: Circle;

let exact: Box<Circle> = Box<Circle> { value: circle };
const widened: &'static Box<Shape> = &exact;

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

struct Box<Value> {
/// @generic.template symbol=Box parameters=(out Value)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out Value)
/// @definition.field symbol=Box.value source="value: Value" key=value type=Value
/// @type.symbol symbol=Box.Value source=Value type=Value

    value: Value;
    /// @type.symbol symbol=Box.value source="value: Value" type=Value
    /// @resolution.name source=Value target=Box.Value

}

declare const circle: Circle;
/// @type.symbol symbol=circle source=circle type=Circle
/// @resolution.pattern source=circle kind=binding target=circle
/// @resolution.name source=Circle target=Circle

let exact: Box<Circle> = Box { value: circle };
/// @type.symbol symbol=exact source=exact type=Box<Circle>
/// @resolution.pattern source=exact kind=binding target=exact
/// @resolution.name source=Box target=Box
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Box target=Box
/// @resolution.name source=circle target=circle
/// @resolution.place source=circle placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circle root=circle

const widened: &Box<Shape> = &exact;
/// @type.symbol symbol=widened source=widened type=&'static Box<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Box target=Box
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=exact target=exact
/// @resolution.place source=exact placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=exact root=exact
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '&'static Box<Circle>' is not assignable to type '&'static Box<Shape>'"
/// @diagnostic.label line=12 column=30 span="&exact" line_source="const widened: &Box<Shape> = &exact;"
/// @diagnostic.related line=12 column=16 span="&" line_source="const widened: &Box<Shape> = &exact;" message="expected due to this annotation"
"#,
    );
}

/// Reject returning a result at a widened error argument.
#[test]
fn test_reject_a_result_stored_across_a_widened_error_argument() {
    let session = TestSession::single(
        r#"
function widen<U, E, F>(result: Result<U, F>): Result<U, E | F> {
    return result;
}
"#,
    );

    session.assert_diagnostics(
        session.dir_checked_key("main.ds"),
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'Result<U, F>' is not assignable to the declared result type 'Result<U, E | F>'"
/// @diagnostic.label line=3 column=12 span="result" line_source="return result;"
/// @diagnostic.note message="the mismatch is in type argument 1 of 'Result': expected 'E | F', found 'F'"
"#,
    );
}
