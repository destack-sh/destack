use crate::tests::{DirRows, TestSession};

/// A struct widens a subclass type argument to its base class.
#[test]
fn test_widen_a_subclass_type_argument_to_its_base_class() {
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

    session.assert_dir(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
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
/// @generic.instance id=Holder<Circle> template=Holder arguments=(Circle)
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const shapes: Holder<Shape> = circles;
/// @type.symbol symbol=shapes source=shapes type=Holder<Shape>
/// @resolution.pattern source=shapes kind=binding target=shapes
/// @generic.instance id=Holder<Shape> template=Holder arguments=(Shape)
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
    );
}

/// Widening a type argument into a union reports a diagnostic.
#[test]
fn test_reject_widening_a_type_argument_into_a_union() {
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

    session.assert_dir_and_diagnostics(
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
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<Circle>' is not assignable to type 'Holder<Circle | Square>'"
/// @diagnostic.label line=11 column=41 span="circles" line_source="const either: Holder<Circle | Square> = circles;"
/// @diagnostic.related line=11 column=15 span="Holder" line_source="const either: Holder<Circle | Square> = circles;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'Circle | Square', found 'Circle'"
"#,
    );
}

/// Widening a literal type argument to its scalar reports a diagnostic.
#[test]
fn test_reject_widening_a_literal_type_argument() {
    let session = TestSession::single(
        r#"
struct Holder<T> {
    value: T;
}

declare const one: Holder<1>;
const wide: Holder<int32> = one;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Holder<out T> {
    value: T;
}

declare const one: Holder<1>;
const wide: Holder<int32> = one;

=== dir ===
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
/// @resolution.place source=one placement="local" lifetime="static" access="immutable"
/// @resolution.access source=one root=one
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<1>' is not assignable to type 'Holder<int32>'"
/// @diagnostic.label line=7 column=29 span="one" line_source="const wide: Holder<int32> = one;"
/// @diagnostic.related line=7 column=13 span="Holder" line_source="const wide: Holder<int32> = one;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'int32', found '1'"
"#,
    );
}

/// Erasing a type argument to unknown reports a diagnostic.
#[test]
fn test_move_a_struct_argument_into_unknown() {
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
class Circle {}
/// @type.symbol symbol=Circle source="class Circle {}" type=typeof Circle
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
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
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
fn test_reject_aliasing_a_class_argument_as_unknown() {
    let session = TestSession::single(
        r#"
class Circle {}

class Holder<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

declare const circles: Holder<Circle>;
const opaque: Holder<unknown> = circles;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Circle {}

class Holder<in out T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

declare const circles: Holder<Circle>;
const opaque: Holder<unknown> = circles;

=== dir ===
class Circle {}
/// @type.symbol symbol=Circle source="class Circle {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle {}"

class Holder<T> {
/// @generic.template symbol=Holder parameters=(in out T)
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder template=(in out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=(this: &'managed Holder<T>, T) => Holder<T>
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

    constructor(value: T) {
    /// @type.symbol symbol=Holder.constructor type=(this: &'managed Holder<T>, T) => Holder<T>
    /// @type.symbol symbol=Holder.constructor.this type=&'managed Holder<T>
    /// @type.symbol symbol=Holder.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Holder type=&'managed Holder<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Holder<T>, target=field(receiver=&'managed Holder<T>, target=Holder.value, type=T), type=T" type=T
        /// @resolution.name source=value target=Holder.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Holder.constructor.value

    }
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
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<Circle>' is not assignable to type 'Holder<unknown>'"
/// @diagnostic.label line=13 column=33 span="circles" line_source="const opaque: Holder<unknown> = circles;"
/// @diagnostic.related line=13 column=15 span="Holder" line_source="const opaque: Holder<unknown> = circles;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'unknown', found 'Circle'"
"#,
    );
}

/// Boxing a type argument into a dynamic reports a diagnostic.
#[test]
fn test_reject_boxing_a_type_argument_into_a_dynamic() {
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
interface Draw {}
/// @generic.template symbol=Draw parameters=(this: Draw)
/// @type.symbol symbol=Draw source="interface Draw {}" type=Draw
/// @definition.interface symbol=Draw source="interface Draw {}" template=(this: Draw)
/// @definition.where symbol=Draw source="interface Draw {}" relation=satisfies left=this right=Draw

class Circle implements Draw {}
/// @type.symbol symbol=Circle source="class Circle implements Draw {}" type=typeof Circle
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
/// @resolution.name source=Dynamic target=Dynamic
/// @resolution.name source=Draw target=Draw
/// @resolution.name source=circles target=circles
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<Circle>' is not assignable to type 'Holder<Dynamic<Draw>>'"
/// @diagnostic.label line=10 column=40 span="circles" line_source="const dynamic: Holder<Dynamic<Draw>> = circles;"
/// @diagnostic.related line=10 column=16 span="Holder" line_source="const dynamic: Holder<Dynamic<Draw>> = circles;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'Dynamic<Draw>', found 'Circle'"
"#,
    );
}

/// A function-typed argument widens its result to a base class.
#[test]
fn test_widen_a_function_result_inside_a_type_argument() {
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

    session.assert_dir(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
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
/// @type.symbol symbol=makers source=makers type=Holder<() => Circle>
/// @resolution.pattern source=makers kind=binding target=makers
/// @generic.instance id="Holder<() => Circle>" template=Holder arguments=(() => Circle)
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const widened: Holder<() => Shape> = makers;
/// @type.symbol symbol=widened source=widened type=Holder<() => Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @generic.instance id="Holder<() => Shape>" template=Holder arguments=(() => Shape)
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=makers target=makers
/// @resolution.place source=makers placement="local" lifetime="static" access="immutable"
/// @resolution.access source=makers root=makers
"#,
    );
}

/// Widening a function-typed argument's result into a union reports a diagnostic.
#[test]
fn test_reject_a_union_function_result_inside_a_type_argument() {
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

    session.assert_dir_and_diagnostics(
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
/// @type.symbol symbol=makers source=makers type=Holder<() => Circle>
/// @resolution.pattern source=makers kind=binding target=makers
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle

const either: Holder<() => Circle | Square> = makers;
/// @type.symbol symbol=either source=either type=Holder<() => Circle | Square>
/// @resolution.pattern source=either kind=binding target=either
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=makers target=makers
/// @resolution.place source=makers placement="local" lifetime="static" access="immutable"
/// @resolution.access source=makers root=makers
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Holder<() => Circle>' is not assignable to type 'Holder<() => Circle | Square>'"
/// @diagnostic.label line=11 column=47 span="makers" line_source="const either: Holder<() => Circle | Square> = makers;"
/// @diagnostic.related line=11 column=15 span="Holder" line_source="const either: Holder<() => Circle | Square> = makers;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected '() => Circle | Square', found '() => Circle'"
"#,
    );
}

/// A managed class keeps its type arguments invariant.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare class Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=typeof Box
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
/// @resolution.place source=boxed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=boxed root=boxed
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Box<Circle>' is not assignable to type 'Box<Shape>'"
/// @diagnostic.label line=10 column=29 span="boxed" line_source="const widened: Box<Shape> = boxed;"
/// @diagnostic.related line=10 column=16 span="Box" line_source="const widened: Box<Shape> = boxed;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Box': expected 'Shape', found 'Circle'"
"#,
    );
}

/// An aliased class with only readonly fields widens its type argument.
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

    session.assert_dir(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare class Label<T> {
/// @generic.template symbol=Label parameters=(out T)
/// @type.symbol symbol=Label type=typeof Label
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
/// @generic.instance id=Label<Circle> template=Label arguments=(Circle)
/// @resolution.name source=Label target=Label
/// @resolution.name source=Circle target=Circle

const widened: Label<Shape> = labeled;
/// @type.symbol symbol=widened source=widened type=Label<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @generic.instance id=Label<Shape> template=Label arguments=(Shape)
/// @resolution.name source=Label target=Label
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=labeled target=labeled
/// @resolution.place source=labeled placement="local" lifetime="static" access="immutable"
/// @resolution.access source=labeled root=labeled
"#,
    );
}

/// An owned handle to a field-only class widens its type argument.
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

    session.assert_dir(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare class Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

declare const boxed: ^Box<Circle>;
/// @type.symbol symbol=boxed source=boxed type=^Box<Circle>
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @generic.instance id=Box<Circle> template=Box arguments=(Circle)
/// @resolution.name source=Box target=Box
/// @resolution.name source=Circle target=Circle

const widened: ^Box<Shape> = boxed;
/// @type.symbol symbol=widened source=widened type=^Box<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @generic.instance id=Box<Shape> template=Box arguments=(Shape)
/// @resolution.name source=Box target=Box
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=boxed target=boxed
/// @resolution.place source=boxed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=boxed root=boxed
"#,
    );
}

/// Widening an owned class with a method taking the parameter reports a diagnostic.
#[test]
fn test_reject_widening_an_owned_class_with_an_invariant_method() {
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare class Pipe<T> {
/// @generic.template symbol=Pipe parameters=(in out T)
/// @type.symbol symbol=Pipe type=typeof Pipe
/// @definition.class symbol=Pipe template=(in out T)
/// @definition.field symbol=Pipe.store source="store: T" key=store type=T
/// @definition.method symbol=Pipe.put source="put(this, value: T): void" slot=put type=(this: Pipe<T>, T) => void
/// @type.symbol symbol=Pipe.T source=T type=T

    store: T;
    /// @type.symbol symbol=Pipe.store source="store: T" type=T
    /// @resolution.name source=T target=Pipe.T

    put(this, value: T): void;
    /// @type.symbol symbol=Pipe.put source="put(this, value: T): void" type=(this: Pipe<T>, T) => void
    /// @type.symbol symbol=Pipe.put.this source=this type=Pipe<T>
    /// @type.symbol symbol=Pipe.put.value source="value: T" type=T
    /// @resolution.name source=T target=Pipe.T

}

declare const pipe: ^Pipe<Circle>;
/// @type.symbol symbol=pipe source=pipe type=^Pipe<Circle>
/// @resolution.pattern source=pipe kind=binding target=pipe
/// @resolution.name source=Pipe target=Pipe
/// @resolution.name source=Circle target=Circle

const widened: ^Pipe<Shape> = pipe;
/// @type.symbol symbol=widened source=widened type=^Pipe<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=Pipe target=Pipe
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=pipe target=pipe
/// @resolution.place source=pipe placement="local" lifetime="static" access="immutable"
/// @resolution.access source=pipe root=pipe
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '^Pipe<Circle>' is not assignable to type '^Pipe<Shape>'"
/// @diagnostic.label line=12 column=31 span="pipe" line_source="const widened: ^Pipe<Shape> = pipe;"
/// @diagnostic.related line=12 column=16 span="^" line_source="const widened: ^Pipe<Shape> = pipe;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Pipe': expected 'Shape', found 'Circle'"
"#,
    );
}

/// An aliased class stays invariant when an extension writes its parameter.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Stack<T> {
/// @generic.template symbol=Stack parameters=(in out T#1)
/// @type.symbol symbol=Stack type=typeof Stack
/// @definition.class symbol=Stack template=(in out T#1)
/// @definition.field symbol=Stack.items source="items: T[] = []" key=items type=T#1[]
/// @type.symbol symbol=Stack.T source=T type=T#1

    items: T[] = [];
    /// @type.symbol symbol=Stack.items source="items: T[] = []" type=T#1[]
    /// @resolution.name source=T target=Stack.T
    /// @resolution.call source=[] parameters=(^Slice<T#1>) arguments=(rest() as T#1) return=T#1[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<T#1>
    /// @generic.instantiation id=arrayFromOwnedSlice<T#1> template=arrayFromOwnedSlice arguments=(T#1) owner=Stack

}

extension<T> of Stack<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Stack<T#2>
/// @definition.method symbol=refill slot=refill type=(this: Stack<T#2>, T#2) => void
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=T target=T

    refill(this, value: T): void {
    /// @type.symbol symbol=refill type=(this: Stack<T#2>, T#2) => void
    /// @type.symbol symbol=refill.this source=this type=Stack<T#2>
    /// @type.symbol symbol=refill.value source="value: T" type=T#2
    /// @resolution.name source=T target=T

        this.items = [value];
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Stack<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.items kind=place
        /// @resolution.place source=this.items placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.items root=this keys=[items]
        /// @resolution.assignment source=this.items write="receiver=Stack<T#2>, target=field(receiver=Stack<T#2>, target=Stack.items, type=T#2[]), type=T#2[]" type=T#2[]
        /// @resolution.call source=[value] parameters=(^Slice<T#2>) arguments=(rest(provided(value) as T#2) as T#2) return=T#2[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<T#2>
        /// @generic.instantiation id=arrayFromOwnedSlice<T#2> template=arrayFromOwnedSlice arguments=(T#2) owner=refill
        /// @resolution.name source=value target=refill.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=refill.value

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
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Stack<Circle>' is not assignable to type 'Stack<Shape>'"
/// @diagnostic.label line=16 column=31 span="circles" line_source="const widened: Stack<Shape> = circles;"
/// @diagnostic.related line=16 column=16 span="Stack" line_source="const widened: Stack<Shape> = circles;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Stack': expected 'Shape', found 'Circle'"
"#,
    );
}

/// A readonly view of that class widens its type argument.
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

    session.assert_dir(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Stack<T> {
/// @generic.template symbol=Stack parameters=(in out T#1)
/// @type.symbol symbol=Stack type=typeof Stack
/// @definition.class symbol=Stack template=(in out T#1)
/// @definition.field symbol=Stack.items source="items: T[] = []" key=items type=T#1[]
/// @type.symbol symbol=Stack.T source=T type=T#1

    items: T[] = [];
    /// @type.symbol symbol=Stack.items source="items: T[] = []" type=T#1[]
    /// @generic.instance id=Array<T#1> template=Array arguments=(T#1)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<T#1>> template=sliceAssumeInit arguments=(MaybeUninit<T#1>)
    /// @generic.instance id=sliceUninit<MaybeUninit<T#1>> template=sliceUninit arguments=(MaybeUninit<T#1>)
    /// @resolution.name source=T target=Stack.T
    /// @resolution.call source=[] parameters=(^Slice<T#1>) arguments=(rest() as T#1) return=T#1[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<T#1>
    /// @generic.instantiation id=arrayFromOwnedSlice<T#1> template=arrayFromOwnedSlice arguments=(T#1) owner=Stack
    /// @generic.instance id=arrayFromOwnedSlice<T#1> template=arrayFromOwnedSlice arguments=(T#1)

}

extension<T> of Stack<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @generic.instance id=Stack<T#2> template=Stack arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Stack<T#2>
/// @definition.method symbol=refill slot=refill type=(this: Stack<T#2>, T#2) => void
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=T target=T

    refill(this, value: T): void {
    /// @type.symbol symbol=refill type=(this: Stack<T#2>, T#2) => void
    /// @type.symbol symbol=refill.this source=this type=Stack<T#2>
    /// @type.symbol symbol=refill.value source="value: T" type=T#2
    /// @resolution.name source=T target=T

        this.items = [value];
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Stack<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.items kind=place
        /// @resolution.place source=this.items placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.items root=this keys=[items]
        /// @resolution.assignment source=this.items write="receiver=Stack<T#2>, target=field(receiver=Stack<T#2>, target=Stack.items, type=T#2[]), type=T#2[]" type=T#2[]
        /// @generic.instance id=Array<T#2> template=Array arguments=(T#2)
        /// @generic.instance id=sliceAssumeInit<MaybeUninit<T#2>> template=sliceAssumeInit arguments=(MaybeUninit<T#2>)
        /// @generic.instance id=sliceUninit<MaybeUninit<T#2>> template=sliceUninit arguments=(MaybeUninit<T#2>)
        /// @resolution.call source=[value] parameters=(^Slice<T#2>) arguments=(rest(provided(value) as T#2) as T#2) return=T#2[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<T#2>
        /// @generic.instantiation id=arrayFromOwnedSlice<T#2> template=arrayFromOwnedSlice arguments=(T#2) owner=refill
        /// @generic.instance id=arrayFromOwnedSlice<T#2> template=arrayFromOwnedSlice arguments=(T#2)
        /// @resolution.name source=value target=refill.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=refill.value

    }
}

declare const circles: Stack<Circle>;
/// @type.symbol symbol=circles source=circles type=Stack<Circle>
/// @resolution.pattern source=circles kind=binding target=circles
/// @generic.instance id=Stack<Circle> template=Stack arguments=(Circle)
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=Circle target=Circle

const view: readonly Stack<Shape> = circles;
/// @type.symbol symbol=view source=view type=readonly Stack<Shape>
/// @resolution.pattern source=view kind=binding target=view
/// @generic.instance id=Stack<Shape> template=Stack arguments=(Shape)
/// @resolution.name source=Stack target=Stack
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
    );
}

/// A method declared on the class keeps even a readonly view invariant.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

class Bag<T> {
/// @generic.template symbol=Bag parameters=(in out T)
/// @type.symbol symbol=Bag type=typeof Bag
/// @definition.class symbol=Bag template=(in out T)
/// @definition.field symbol=Bag.items source="items: T[] = []" key=items type=T[]
/// @definition.method symbol=Bag.refill slot=refill type=(this: Bag<T>, T) => void
/// @type.symbol symbol=Bag.T source=T type=T

    items: T[] = [];
    /// @type.symbol symbol=Bag.items source="items: T[] = []" type=T[]
    /// @resolution.name source=T target=Bag.T
    /// @resolution.call source=[] parameters=(^Slice<T>) arguments=(rest() as T) return=T[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<T>
    /// @generic.instantiation id=arrayFromOwnedSlice<T> template=arrayFromOwnedSlice arguments=(T) owner=Bag

    refill(this, value: T): void {
    /// @type.symbol symbol=Bag.refill type=(this: Bag<T>, T) => void
    /// @type.symbol symbol=Bag.refill.this source=this type=Bag<T>
    /// @type.symbol symbol=Bag.refill.value source="value: T" type=T
    /// @resolution.name source=T target=Bag.T

        this.items = [value];
        /// @resolution.receiver source=this kind=this declaration=Bag type=Bag<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.items kind=place
        /// @resolution.place source=this.items placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.items root=this keys=[items]
        /// @resolution.assignment source=this.items write="receiver=Bag<T>, target=field(receiver=Bag<T>, target=Bag.items, type=T[]), type=T[]" type=T[]
        /// @resolution.call source=[value] parameters=(^Slice<T>) arguments=(rest(provided(value) as T) as T) return=T[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<T>
        /// @generic.instantiation id=arrayFromOwnedSlice<T> template=arrayFromOwnedSlice arguments=(T) owner=Bag.refill
        /// @resolution.name source=value target=Bag.refill.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Bag.refill.value

    }
}

declare const circles: Bag<Circle>;
/// @type.symbol symbol=circles source=circles type=Bag<Circle>
/// @resolution.pattern source=circles kind=binding target=circles
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=Circle target=Circle

const view: readonly Bag<Shape> = circles;
/// @type.symbol symbol=view source=view type=readonly Bag<Shape>
/// @resolution.pattern source=view kind=binding target=view
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles
/// @resolution.place source=circles placement="local" lifetime="static" access="immutable"
/// @resolution.access source=circles root=circles
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Bag<Circle>' is not assignable to type 'readonly Bag<Shape>'"
/// @diagnostic.label line=14 column=35 span="circles" line_source="const view: readonly Bag<Shape> = circles;"
/// @diagnostic.related line=14 column=13 span="readonly" line_source="const view: readonly Bag<Shape> = circles;" message="expected due to this annotation"
"#,
    );
}

/// An intrinsic newtype declared covariant widens its argument to a base class.
#[test]
fn test_widen_an_intrinsic_newtype_argument_to_a_base_class() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

newtype Handle<out T> = intrinsic;

declare const handle: Handle<Circle>;
const widened: Handle<Shape> = handle;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

newtype Handle<out T> = intrinsic;

declare const handle: Handle<Circle>;
const widened: Handle<Shape> = handle;

=== dir ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=typeof Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=typeof Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

newtype Handle<out T> = intrinsic;
/// @generic.template symbol=Handle parameters=(out T)
/// @type.symbol symbol=Handle source="newtype Handle<out T> = intrinsic" type=Handle
/// @definition.newtype symbol=Handle source="newtype Handle<out T> = intrinsic" template=(out T) backing=intrinsic constructors=[<out T>(intrinsic) => Handle<T>]
/// @type.symbol symbol=Handle.T source="out T" type=T

declare const handle: Handle<Circle>;
/// @type.symbol symbol=handle source=handle type=Handle<Circle>
/// @resolution.pattern source=handle kind=binding target=handle
/// @generic.instance id=Handle<Circle> template=Handle arguments=(Circle)
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=Circle target=Circle

const widened: Handle<Shape> = handle;
/// @type.symbol symbol=widened source=widened type=Handle<Shape>
/// @resolution.pattern source=widened kind=binding target=widened
/// @generic.instance id=Handle<Shape> template=Handle arguments=(Shape)
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=handle target=handle
/// @resolution.place source=handle placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handle root=handle
"#,
    );
}

/// Widening a covariant newtype argument into a union reports a diagnostic.
#[test]
fn test_reject_a_union_argument_on_an_intrinsic_newtype() {
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

    session.assert_dir_and_diagnostics(
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

newtype Handle<out T> = intrinsic;
/// @generic.template symbol=Handle parameters=(out T)
/// @type.symbol symbol=Handle source="newtype Handle<out T> = intrinsic" type=Handle
/// @definition.newtype symbol=Handle source="newtype Handle<out T> = intrinsic" template=(out T) backing=intrinsic constructors=[<out T>(intrinsic) => Handle<T>]
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
/// @resolution.place source=handle placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handle root=handle
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Handle<Circle>' is not assignable to type 'Handle<Circle | Square>'"
/// @diagnostic.label line=9 column=41 span="handle" line_source="const either: Handle<Circle | Square> = handle;"
/// @diagnostic.related line=9 column=15 span="Handle" line_source="const either: Handle<Circle | Square> = handle;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Handle': expected 'Circle | Square', found 'Circle'"
"#,
    );
}

/// A readonly borrow widens its argument to a base class and rejects a union.
#[test]
fn test_widen_a_readonly_borrow_to_a_base_class_but_not_to_a_union() {
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

    session.assert_dir_and_diagnostics(
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
const view: &'static readonly Holder<Shape> = &readonly holder;
const either: &'static readonly Holder<Circle | Square> = &readonly holder;

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
/// @type.symbol symbol=view source=view type=&'static readonly Holder<Shape>
/// @resolution.pattern source=view kind=binding target=view
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=holder target=holder
/// @resolution.place source=holder placement="local" lifetime="static" access="immutable"
/// @resolution.access source=holder root=holder

const either: &readonly Holder<Circle | Square> = &readonly holder;
/// @type.symbol symbol=either source=either type=&'static readonly Holder<Circle | Square>
/// @resolution.pattern source=either kind=binding target=either
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square
/// @resolution.name source=holder target=holder
/// @resolution.place source=holder placement="local" lifetime="static" access="immutable"
/// @resolution.access source=holder root=holder
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '&'static readonly Holder<Circle>' is not assignable to type '&'static readonly Holder<Circle | Square>'"
/// @diagnostic.label line=12 column=51 span="&readonly holder" line_source="const either: &readonly Holder<Circle | Square> = &readonly holder;"
/// @diagnostic.related line=12 column=15 span="&" line_source="const either: &readonly Holder<Circle | Square> = &readonly holder;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Holder': expected 'Circle | Square', found 'Circle'"
"#,
    );
}
