use crate::tests::{DirRows, TestSession};

#[test]
fn test_argument_upcast_widens_owned_copies() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

struct Holder<T> {
    value: T;
}

declare const circles: Holder<Circle>;
const shapes: Holder<Shape> = circles;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

struct Holder<out T> {
    value: T;
}

declare const circles: Holder<Circle>;
const shapes: Holder<Shape> = circles;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

struct Holder<T> {
/// @generic.template symbol=Holder parameters=(out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

declare const circles: Holder<Circle>;
/// @type.symbol symbol=circles source=circles type=Holder<Circle>
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const shapes: Holder<Shape> = circles;
/// @type.symbol symbol=shapes source=shapes type=Holder<Shape>
/// @resolution.pattern source=shapes kind=binding target=shapes
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles

/// @generic.instance id=Holder<Circle> template=Holder arguments=(Circle)
/// @generic.instance id=Holder<Shape> template=Holder arguments=(Shape)
"#,
    );
}

#[test]
fn test_argument_union_injection_is_rejected() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}
class Square extends Shape {}

struct Holder<T> {
    value: T;
}

declare const circles: Holder<Circle>;
const either: Holder<Circle | Square> = circles;
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

struct Holder<out T> {
    value: T;
}

declare const circles: Holder<Circle>;
const either: Holder<Circle | Square> = circles;

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

struct Holder<T> {
/// @generic.template symbol=Holder parameters=(out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

declare const circles: Holder<Circle>;
/// @type.symbol symbol=circles source=circles type=Holder<Circle>
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const either: Holder<Circle | Square> = circles;
/// @type.symbol symbol=either source=either type=Holder<Circle | Square>
/// @resolution.pattern source=either kind=binding target=either
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=circles target=circles

/// @generic.instance id="Holder<Circle | Square>" template=Holder arguments=(Circle | Square)
/// @generic.instance id=Holder<Circle> template=Holder arguments=(Circle)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<Circle>' is not assignable to type 'Holder<Circle | Square>'"
/// @diagnostic.label line=11 column=41 span="circles" line_source="const either: Holder<Circle | Square> = circles;"
/// @diagnostic.related line=11 column=15 span="Holder" line_source="const either: Holder<Circle | Square> = circles;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'Circle | Square', found 'Circle'"
"#,
    );
}

#[test]
fn test_argument_literal_widening_is_rejected() {
    let session = TestSession::single(
        r#"
struct Holder<T> {
    value: T;
}

declare const one: Holder<1>;
const wide: Holder<int32> = one;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Holder<out T> {
    value: T;
}

declare const one: Holder<1>;
const wide: Holder<int32> = one;

=== checked ===
struct Holder<T> {
/// @generic.template symbol=Holder parameters=(out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

declare const one: Holder<1>;
/// @type.symbol symbol=one source=one type=Holder<1>
/// @resolution.pattern source=one kind=binding target=one
/// @resolution.name source=Holder target=Holder

const wide: Holder<int32> = one;
/// @type.symbol symbol=wide source=wide type=Holder<int32>
/// @resolution.pattern source=wide kind=binding target=wide
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=one target=one

/// @generic.instance id=Holder<1> template=Holder arguments=(1)
/// @generic.instance id=Holder<int32> template=Holder arguments=(int32)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<1>' is not assignable to type 'Holder<int32>'"
/// @diagnostic.label line=7 column=29 span="one" line_source="const wide: Holder<int32> = one;"
/// @diagnostic.related line=7 column=13 span="Holder" line_source="const wide: Holder<int32> = one;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'int32', found '1'"
"#,
    );
}

#[test]
fn test_argument_existential_erasure_is_rejected() {
    let session = TestSession::single(
        r#"
class Circle {}

struct Holder<T> {
    value: T;
}

declare const circles: Holder<Circle>;
const opaque: Holder<unknown> = circles;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Circle {}

struct Holder<out T> {
    value: T;
}

declare const circles: Holder<Circle>;
const opaque: Holder<unknown> = circles;

=== checked ===
class Circle {}
/// @type.symbol symbol=Circle source="class Circle {}" type=Circle
/// @definition.class symbol=Circle source="class Circle {}"

struct Holder<T> {
/// @generic.template symbol=Holder parameters=(out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

declare const circles: Holder<Circle>;
/// @type.symbol symbol=circles source=circles type=Holder<Circle>
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const opaque: Holder<unknown> = circles;
/// @type.symbol symbol=opaque source=opaque type=Holder<unknown>
/// @resolution.pattern source=opaque kind=binding target=opaque
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=circles target=circles

/// @generic.instance id=Holder<Circle> template=Holder arguments=(Circle)
/// @generic.instance id=Holder<unknown> template=Holder arguments=(unknown)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<Circle>' is not assignable to type 'Holder<unknown>'"
/// @diagnostic.label line=9 column=33 span="circles" line_source="const opaque: Holder<unknown> = circles;"
/// @diagnostic.related line=9 column=15 span="Holder" line_source="const opaque: Holder<unknown> = circles;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'unknown', found 'Circle'"
"#,
    );
}

#[test]
fn test_argument_dynamic_erection_is_rejected() {
    let session = TestSession::single(
        r#"
interface Draw {}
class Circle implements Draw {}

struct Holder<T> {
    value: T;
}

declare const circles: Holder<Circle>;
const dynamic: Holder<Dynamic<Draw>> = circles;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Draw {}
class Circle implements Draw {}

struct Holder<out T> {
    value: T;
}

declare const circles: Holder<Circle>;
const dynamic: Holder<Dynamic<Draw>> = circles;

=== checked ===
interface Draw {}
/// @type.symbol symbol=Draw source="interface Draw {}" type=Draw
/// @definition.interface symbol=Draw source="interface Draw {}"

class Circle implements Draw {}
/// @type.symbol symbol=Circle source="class Circle implements Draw {}" type=Circle
/// @definition.class symbol=Circle source="class Circle implements Draw {}"
/// @definition.where symbol=Circle source=Draw relation=satisfies left=this right=Draw
/// @definition.implements symbol=Circle source=Draw target=Draw
/// @resolution.name source=Draw target=Draw

struct Holder<T> {
/// @generic.template symbol=Holder parameters=(out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

declare const circles: Holder<Circle>;
/// @type.symbol symbol=circles source=circles type=Holder<Circle>
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const dynamic: Holder<Dynamic<Draw>> = circles;
/// @type.symbol symbol=dynamic source=dynamic type=Holder<Dynamic<Draw>>
/// @resolution.pattern source=dynamic kind=binding target=dynamic
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Dynamic target=memory.dynamic.Dynamic
/// @resolution.name source=Draw target=Draw
/// @resolution.name source=circles target=circles

/// @generic.instance id=Dynamic<Draw> template=memory.dynamic.Dynamic arguments=(Draw)
/// @generic.instance id=Holder<Circle> template=Holder arguments=(Circle)
/// @generic.instance id=Holder<Dynamic<Draw>> template=Holder arguments=(Dynamic<Draw>)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<Circle>' is not assignable to type 'Holder<Dynamic<Draw>>'"
/// @diagnostic.label line=10 column=40 span="circles" line_source="const dynamic: Holder<Dynamic<Draw>> = circles;"
/// @diagnostic.related line=10 column=16 span="Holder" line_source="const dynamic: Holder<Dynamic<Draw>> = circles;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'Dynamic<Draw>', found 'Circle'"
"#,
    );
}

#[test]
fn test_argument_function_interiors_widen_identity_edges() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

struct Holder<T> {
    value: T;
}

declare const makers: Holder<() => Circle>;
const widened: Holder<() => Shape> = makers;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

struct Holder<out T> {
    value: T;
}

declare const makers: Holder<() => Circle>;
const widened: Holder<() => Shape> = makers;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

struct Holder<T> {
/// @generic.template symbol=Holder parameters=(out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

declare const makers: Holder<() => Circle>;
/// @type.symbol symbol=makers source=makers type=Holder<Function<(), Circle>>
/// @resolution.pattern source=makers kind=binding target=makers
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const widened: Holder<() => Shape> = makers;
/// @type.symbol symbol=widened source=widened type=Holder<Function<(), Shape>>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=makers target=makers

/// @generic.instance id="Holder<Function<(), Circle>>" template=Holder arguments=(Function<(), Circle>)
/// @generic.instance id="Holder<Function<(), Shape>>" template=Holder arguments=(Function<(), Shape>)
"#,
    );
}

#[test]
fn test_argument_function_interior_conversion_is_rejected() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}
class Square extends Shape {}

struct Holder<T> {
    value: T;
}

declare const makers: Holder<() => Circle>;
const either: Holder<() => Circle | Square> = makers;
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

struct Holder<out T> {
    value: T;
}

declare const makers: Holder<() => Circle>;
const either: Holder<() => Circle | Square> = makers;

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

struct Holder<T> {
/// @generic.template symbol=Holder parameters=(out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

declare const makers: Holder<() => Circle>;
/// @type.symbol symbol=makers source=makers type=Holder<Function<(), Circle>>
/// @resolution.pattern source=makers kind=binding target=makers
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const either: Holder<() => Circle | Square> = makers;
/// @type.symbol symbol=either source=either type=Holder<Function<(), Circle | Square>>
/// @resolution.pattern source=either kind=binding target=either
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=makers target=makers

/// @generic.instance id="Holder<Function<(), Circle | Square>>" template=Holder arguments=(Function<(), Circle | Square>)
/// @generic.instance id="Holder<Function<(), Circle>>" template=Holder arguments=(Function<(), Circle>)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<() => Circle>' is not assignable to type 'Holder<() => … | …>'"
/// @diagnostic.label line=11 column=47 span="makers" line_source="const either: Holder<() => Circle | Square> = makers;"
/// @diagnostic.related line=11 column=15 span="Holder" line_source="const either: Holder<() => Circle | Square> = makers;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected '() => Circle | Square', found '() => Circle'"
"#,
    );
}

#[test]
fn test_managed_class_arguments_stay_invariant() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare class Box<T> {
    value: T;
}

declare const boxed: Box<Circle>;
const widened: Box<Shape> = boxed;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare class Box<in out T> {
    value: T;
}

declare const boxed: Box<Circle>;
const widened: Box<Shape> = boxed;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare class Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

declare const boxed: Box<Circle>;
/// @type.symbol symbol=boxed source=boxed type=Box<Circle>
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=Box target=Box
/// @resolution.name source=Circle target=Circle

const widened: Box<Shape> = boxed;
/// @type.symbol symbol=widened source=widened type=Box<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Box target=Box
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=boxed target=boxed

/// @generic.instance id=Box<Circle> template=Box arguments=(Circle)
/// @generic.instance id=Box<Shape> template=Box arguments=(Shape)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Box<Circle>' is not assignable to type 'Box<Shape>'"
/// @diagnostic.label line=10 column=29 span="boxed" line_source="const widened: Box<Shape> = boxed;"
/// @diagnostic.related line=10 column=16 span="Box" line_source="const widened: Box<Shape> = boxed;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Box': expected 'Shape', found 'Circle'"
"#,
    );
}

#[test]
fn test_readonly_field_class_widens_under_aliasing() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare class Label<T> {
    readonly value: T;
}

declare const labeled: Label<Circle>;
const widened: Label<Shape> = labeled;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare class Label<out T> {
    readonly value: T;
}

declare const labeled: Label<Circle>;
const widened: Label<Shape> = labeled;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare class Label<T> {
/// @generic.template symbol=Label parameters=(out T)
/// @type.symbol symbol=Label type=Label
/// @definition.class symbol=Label template=(out T)
/// @definition.field symbol=Label.value source="readonly value: T" key=value type=T
/// @type.symbol symbol=Label.T source=T type=T

    readonly value: T;
    /// @type.symbol symbol=Label.value source="readonly value: T" type=T
    /// @resolution.name source=T target=Label.T

}

declare const labeled: Label<Circle>;
/// @type.symbol symbol=labeled source=labeled type=Label<Circle>
/// @resolution.pattern source=labeled kind=binding target=labeled
/// @resolution.name source=Label target=Label
/// @resolution.name source=Circle target=Circle

const widened: Label<Shape> = labeled;
/// @type.symbol symbol=widened source=widened type=Label<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Label target=Label
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=labeled target=labeled

/// @generic.instance id=Label<Circle> template=Label arguments=(Circle)
/// @generic.instance id=Label<Shape> template=Label arguments=(Shape)
"#,
    );
}

#[test]
fn test_owned_handle_widens_field_only_class() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare class Box<T> {
    value: T;
}

declare const boxed: ^Box<Circle>;
const widened: ^Box<Shape> = boxed;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare class Box<in out T> {
    value: T;
}

declare const boxed: ^Box<Circle>;
const widened: ^Box<Shape> = boxed;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare class Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

declare const boxed: ^Box<Circle>;
/// @type.symbol symbol=boxed source=boxed type=Owned<Box<Circle>>
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=Box target=Box
/// @resolution.name source=Circle target=Circle

const widened: ^Box<Shape> = boxed;
/// @type.symbol symbol=widened source=widened type=Owned<Box<Shape>>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Box target=Box
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=boxed target=boxed

/// @generic.instance id=Box<Circle> template=Box arguments=(Circle)
/// @generic.instance id=Box<Shape> template=Box arguments=(Shape)
"#,
    );
}

#[test]
fn test_owned_handle_pins_carried_methods() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare class Pipe<T> {
    store: T;

    put(this, value: T): void;
}

declare const pipe: ^Pipe<Circle>;
const widened: ^Pipe<Shape> = pipe;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare class Pipe<in out T> {
    store: T;

    put(this, value: T): void;
}

declare const pipe: ^Pipe<Circle>;
const widened: ^Pipe<Shape> = pipe;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare class Pipe<T> {
/// @generic.template symbol=Pipe parameters=(in out T)
/// @type.symbol symbol=Pipe type=Pipe
/// @definition.class symbol=Pipe template=(in out T)
/// @definition.field symbol=Pipe.store source="store: T" key=store type=T
/// @definition.method symbol=Pipe.put source="put(this, value: T): void" slot=put type=(this: this, T) => void
/// @type.symbol symbol=Pipe.T source=T type=T

    store: T;
    /// @type.symbol symbol=Pipe.store source="store: T" type=T
    /// @resolution.name source=T target=Pipe.T

    put(this, value: T): void;
    /// @type.symbol symbol=Pipe.put source="put(this, value: T): void" type=(this: this, T) => void
    /// @type.symbol symbol=Pipe.put.this source=this type=this
    /// @type.symbol symbol=Pipe.put.value source="value: T" type=T
    /// @resolution.name source=T target=Pipe.T

}

declare const pipe: ^Pipe<Circle>;
/// @type.symbol symbol=pipe source=pipe type=Owned<Pipe<Circle>>
/// @resolution.pattern source=pipe kind=binding target=pipe
/// @resolution.name source=Pipe target=Pipe
/// @resolution.name source=Circle target=Circle

const widened: ^Pipe<Shape> = pipe;
/// @type.symbol symbol=widened source=widened type=Owned<Pipe<Shape>>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Pipe target=Pipe
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=pipe target=pipe

/// @generic.instance id=Pipe<Circle> template=Pipe arguments=(Circle)
/// @generic.instance id=Pipe<Shape> template=Pipe arguments=(Shape)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '^Pipe<Circle>' is not assignable to type '^Pipe<Shape>'"
/// @diagnostic.label line=12 column=31 span="pipe" line_source="const widened: ^Pipe<Shape> = pipe;"
/// @diagnostic.related line=12 column=16 span="^" line_source="const widened: ^Pipe<Shape> = pipe;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Pipe': expected 'Shape', found 'Circle'"
"#,
    );
}

#[test]
fn test_extension_collection_stays_invariant_when_aliased() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

class Stack<T> {
    items: T[] = [];
}

extension<T> of Stack<T> {
    refill(this, value: T): void {
        this.items = [value];
    }
}

declare const circles: Stack<Circle>;
const widened: Stack<Shape> = circles;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

class Stack<in out T> {
    items: T[] = [];
}

extension<T> of Stack<T> {
    refill(this, value: T): void {
        this.items = [value];
    }
}

declare const circles: Stack<Circle>;
const widened: Stack<Shape> = circles;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Stack<T> {
/// @generic.template symbol=Stack parameters=(in out T#1)
/// @type.symbol symbol=Stack type=Stack
/// @definition.class symbol=Stack template=(in out T#1)
/// @definition.field symbol=Stack.items source="items: T[] = []" key=items type=Array<T#1>
/// @type.symbol symbol=Stack.T source=T type=T#1

    items: T[] = [];
    /// @type.symbol symbol=Stack.items source="items: T[] = []" type=Array<T#1>
    /// @resolution.name source=T target=Stack.T

}

extension<T> of Stack<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Stack<T#2>
/// @definition.method symbol=refill slot=refill type=(this: this, T#2) => void
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=T target=T

    refill(this, value: T): void {
    /// @type.symbol symbol=refill type=(this: this, T#2) => void
    /// @type.symbol symbol=refill.this source=this type=this
    /// @type.symbol symbol=refill.value source="value: T" type=T#2
    /// @resolution.name source=T target=T

        this.items = [value];
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Stack<T#2>
        /// @resolution.pattern.assign source=this.items kind=place place=field(Stack.items) type=Array<T#2>
        /// @resolution.name source=value target=refill.value

    }
}

declare const circles: Stack<Circle>;
/// @type.symbol symbol=circles source=circles type=Stack<Circle>
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=Circle target=Circle

const widened: Stack<Shape> = circles;
/// @type.symbol symbol=widened source=widened type=Stack<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles

/// @generic.instance id=Stack<Circle> template=Stack arguments=(Circle)
/// @generic.instance id=Stack<Shape> template=Stack arguments=(Shape)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Stack<Circle>' is not assignable to type 'Stack<Shape>'"
/// @diagnostic.label line=16 column=31 span="circles" line_source="const widened: Stack<Shape> = circles;"
/// @diagnostic.related line=16 column=16 span="Stack" line_source="const widened: Stack<Shape> = circles;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Stack': expected 'Shape', found 'Circle'"
"#,
    );
}

#[test]
fn test_extension_collection_view_is_covariant() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

class Stack<T> {
    items: T[] = [];
}

extension<T> of Stack<T> {
    refill(this, value: T): void {
        this.items = [value];
    }
}

declare const circles: Stack<Circle>;
const view: readonly Stack<Shape> = circles;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

class Stack<in out T> {
    items: T[] = [];
}

extension<T> of Stack<T> {
    refill(this, value: T): void {
        this.items = [value];
    }
}

declare const circles: Stack<Circle>;
const view: readonly Stack<Shape> = circles;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Stack<T> {
/// @generic.template symbol=Stack parameters=(in out T#1)
/// @type.symbol symbol=Stack type=Stack
/// @definition.class symbol=Stack template=(in out T#1)
/// @definition.field symbol=Stack.items source="items: T[] = []" key=items type=Array<T#1>
/// @type.symbol symbol=Stack.T source=T type=T#1

    items: T[] = [];
    /// @type.symbol symbol=Stack.items source="items: T[] = []" type=Array<T#1>
    /// @resolution.name source=T target=Stack.T

}

extension<T> of Stack<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Stack<T#2>
/// @definition.method symbol=refill slot=refill type=(this: this, T#2) => void
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=T target=T

    refill(this, value: T): void {
    /// @type.symbol symbol=refill type=(this: this, T#2) => void
    /// @type.symbol symbol=refill.this source=this type=this
    /// @type.symbol symbol=refill.value source="value: T" type=T#2
    /// @resolution.name source=T target=T

        this.items = [value];
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Stack<T#2>
        /// @resolution.pattern.assign source=this.items kind=place place=field(Stack.items) type=Array<T#2>
        /// @resolution.name source=value target=refill.value

    }
}

declare const circles: Stack<Circle>;
/// @type.symbol symbol=circles source=circles type=Stack<Circle>
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=Circle target=Circle

const view: readonly Stack<Shape> = circles;
/// @type.symbol symbol=view source=view type=Readonly<Stack<Shape>>
/// @resolution.pattern source=view kind=binding target=view
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles

/// @generic.instance id=Stack<Circle> template=Stack arguments=(Circle)
/// @generic.instance id=Stack<Shape> template=Stack arguments=(Shape)
"#,
    );
}

#[test]
fn test_definition_method_blocks_view_covariance() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

class Bag<T> {
    items: T[] = [];

    refill(this, value: T): void {
        this.items = [value];
    }
}

declare const circles: Bag<Circle>;
const view: readonly Bag<Shape> = circles;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

class Bag<in out T> {
    items: T[] = [];

    refill(this, value: T): void {
        this.items = [value];
    }
}

declare const circles: Bag<Circle>;
const view: readonly Bag<Shape> = circles;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Bag<T> {
/// @generic.template symbol=Bag parameters=(in out T)
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag template=(in out T)
/// @definition.field symbol=Bag.items source="items: T[] = []" key=items type=Array<T>
/// @definition.method symbol=Bag.refill slot=refill type=(this: this, T) => void
/// @type.symbol symbol=Bag.T source=T type=T

    items: T[] = [];
    /// @type.symbol symbol=Bag.items source="items: T[] = []" type=Array<T>
    /// @resolution.name source=T target=Bag.T

    refill(this, value: T): void {
    /// @type.symbol symbol=Bag.refill type=(this: this, T) => void
    /// @type.symbol symbol=Bag.refill.this source=this type=this
    /// @type.symbol symbol=Bag.refill.value source="value: T" type=T
    /// @resolution.name source=T target=Bag.T

        this.items = [value];
        /// @resolution.receiver source=this kind=this declaration=Bag type=Bag<T>
        /// @resolution.pattern.assign source=this.items kind=place place=field(Bag.items) type=Array<T>
        /// @resolution.name source=value target=Bag.refill.value

    }
}

declare const circles: Bag<Circle>;
/// @type.symbol symbol=circles source=circles type=Bag<Circle>
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=Circle target=Circle

const view: readonly Bag<Shape> = circles;
/// @type.symbol symbol=view source=view type=Readonly<Bag<Shape>>
/// @resolution.pattern source=view kind=binding target=view
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles

/// @generic.instance id=Bag<Circle> template=Bag arguments=(Circle)
/// @generic.instance id=Bag<Shape> template=Bag arguments=(Shape)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Bag<Circle>' is not assignable to type 'readonly Bag<Shape>'"
/// @diagnostic.label line=14 column=35 span="circles" line_source="const view: readonly Bag<Shape> = circles;"
/// @diagnostic.related line=14 column=13 span="readonly" line_source="const view: readonly Bag<Shape> = circles;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_intrinsic_storage_assertion_widens_identity_edges() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

newtype Handle<out T> = intrinsic;

declare const handle: Handle<Circle>;
const widened: Handle<Shape> = handle;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

newtype Handle<out T> = intrinsic;

declare const handle: Handle<Circle>;
const widened: Handle<Shape> = handle;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

newtype Handle<out T> = intrinsic;
/// @generic.template symbol=Handle parameters=(out T)
/// @type.symbol symbol=Handle source="newtype Handle<out T> = intrinsic" type=Handle
/// @definition.newtype symbol=Handle source="newtype Handle<out T> = intrinsic" template=(out T) backing=intrinsic
/// @type.symbol symbol=Handle.T source="out T" type=T

declare const handle: Handle<Circle>;
/// @type.symbol symbol=handle source=handle type=Handle<Circle>
/// @resolution.pattern source=handle kind=binding target=handle
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=Circle target=Circle

const widened: Handle<Shape> = handle;
/// @type.symbol symbol=widened source=widened type=Handle<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=handle target=handle

/// @generic.instance id=Handle<Circle> template=Handle arguments=(Circle)
/// @generic.instance id=Handle<Shape> template=Handle arguments=(Shape)
"#,
    );
}

#[test]
fn test_intrinsic_storage_assertion_rejects_conversion_edges() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}
class Square extends Shape {}

newtype Handle<out T> = intrinsic;

declare const handle: Handle<Circle>;
const either: Handle<Circle | Square> = handle;
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

newtype Handle<out T> = intrinsic;

declare const handle: Handle<Circle>;
const either: Handle<Circle | Square> = handle;

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

newtype Handle<out T> = intrinsic;
/// @generic.template symbol=Handle parameters=(out T)
/// @type.symbol symbol=Handle source="newtype Handle<out T> = intrinsic" type=Handle
/// @definition.newtype symbol=Handle source="newtype Handle<out T> = intrinsic" template=(out T) backing=intrinsic
/// @type.symbol symbol=Handle.T source="out T" type=T

declare const handle: Handle<Circle>;
/// @type.symbol symbol=handle source=handle type=Handle<Circle>
/// @resolution.pattern source=handle kind=binding target=handle
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=Circle target=Circle

const either: Handle<Circle | Square> = handle;
/// @type.symbol symbol=either source=either type=Handle<Circle | Square>
/// @resolution.pattern source=either kind=binding target=either
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=handle target=handle

/// @generic.instance id="Handle<Circle | Square>" template=Handle arguments=(Circle | Square)
/// @generic.instance id=Handle<Circle> template=Handle arguments=(Circle)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Handle<Circle>' is not assignable to type 'Handle<Circle | Square>'"
/// @diagnostic.label line=9 column=41 span="handle" line_source="const either: Handle<Circle | Square> = handle;"
/// @diagnostic.related line=9 column=15 span="Handle" line_source="const either: Handle<Circle | Square> = handle;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Handle': expected 'Circle | Square', found 'Circle'"
"#,
    );
}

#[test]
fn test_intrinsic_storage_assertion_pins_aliased_payloads() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

newtype Handle<out T> = intrinsic;

declare const handle: Managed<Handle<Circle>>;
const widened: Managed<Handle<Shape>> = handle;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

newtype Handle<out T> = intrinsic;

declare const handle: Managed<Handle<Circle>>;
const widened: Managed<Handle<Shape>> = handle;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

newtype Handle<out T> = intrinsic;
/// @generic.template symbol=Handle parameters=(out T)
/// @type.symbol symbol=Handle source="newtype Handle<out T> = intrinsic" type=Handle
/// @definition.newtype symbol=Handle source="newtype Handle<out T> = intrinsic" template=(out T) backing=intrinsic
/// @type.symbol symbol=Handle.T source="out T" type=T

declare const handle: Managed<Handle<Circle>>;
/// @type.symbol symbol=handle source=handle type=Managed<Handle<Circle>>
/// @resolution.pattern source=handle kind=binding target=handle
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=Circle target=Circle

const widened: Managed<Handle<Shape>> = handle;
/// @type.symbol symbol=widened source=widened type=Managed<Handle<Shape>>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=handle target=handle

/// @generic.instance id=Handle<Circle> template=Handle arguments=(Circle)
/// @generic.instance id=Handle<Shape> template=Handle arguments=(Shape)
/// @generic.instance id=Managed<Handle<Circle>> template=memory.managed.Managed arguments=(Handle<Circle>)
/// @generic.instance id=Managed<Handle<Shape>> template=memory.managed.Managed arguments=(Handle<Shape>)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Managed<Handle<Circle>>' is not assignable to type 'Managed<Handle<Shape>>'"
/// @diagnostic.label line=8 column=41 span="handle" line_source="const widened: Managed<Handle<Shape>> = handle;"
/// @diagnostic.related line=8 column=16 span="Managed" line_source="const widened: Managed<Handle<Shape>> = handle;" message="expected due to this annotation"
/// @diagnostic.note message="'Managed<Handle<Circle>>' reduces to 'Handle<Circle>'"
/// @diagnostic.note message="'Managed<Handle<Shape>>' reduces to 'Handle<Shape>'"
"#,
    );
}

#[test]
fn test_readonly_borrow_payload_widens_identity_edges_only() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}
class Square extends Shape {}

struct Holder<T> {
    value: T;
}

declare const holder: Holder<Circle>;
const view: &readonly Holder<Shape> = &readonly holder;
const either: &readonly Holder<Circle | Square> = &readonly holder;
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

struct Holder<out T> {
    value: T;
}

declare const holder: Holder<Circle>;
const view: Borrowed<Holder<Shape>, "static", "readonly"> = &readonly holder;
const either: Borrowed<Holder<Circle | Square>, "static", "readonly"> = &readonly holder;

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

struct Holder<T> {
/// @generic.template symbol=Holder parameters=(out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

declare const holder: Holder<Circle>;
/// @type.symbol symbol=holder source=holder type=Holder<Circle>
/// @resolution.pattern source=holder kind=binding target=holder
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const view: &readonly Holder<Shape> = &readonly holder;
/// @type.symbol symbol=view source=view type=Borrowed<Holder<Shape>, "static", "readonly">
/// @resolution.pattern source=view kind=binding target=view
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=holder target=holder

const either: &readonly Holder<Circle | Square> = &readonly holder;
/// @type.symbol symbol=either source=either type=Borrowed<Holder<Circle | Square>, "static", "readonly">
/// @resolution.pattern source=either kind=binding target=either
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=holder target=holder

/// @generic.instance id="Holder<Circle | Square>" template=Holder arguments=(Circle | Square)
/// @generic.instance id=Holder<Circle> template=Holder arguments=(Circle)
/// @generic.instance id=Holder<Shape> template=Holder arguments=(Shape)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '&readonly Holder<Circle>' is not assignable to type '&readonly Holder<Circle | Square>'"
/// @diagnostic.label line=12 column=51 span="&readonly holder" line_source="const either: &readonly Holder<Circle | Square> = &readonly holder;"
/// @diagnostic.related line=12 column=15 span="&" line_source="const either: &readonly Holder<Circle | Square> = &readonly holder;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'Circle | Square', found 'Circle'"
"#,
    );
}
