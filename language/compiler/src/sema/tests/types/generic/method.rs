use crate::tests::{DirRows, TestSession};

/// An extension method forwards to a free generic function of the same name.
#[test]
fn test_extension_method_forwards_to_free_generic_function() {
    let session = TestSession::single(
        r#"
interface Scalar {}

function checkedAdd<T: Scalar>(a: T, b: T): T | undefined {
    undefined
}

extension Arithmetic<T: Scalar> of T {
    checkedAdd(other: T): T | undefined {
        checkedAdd(this, other)
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Scalar {}

function checkedAdd<T: Scalar>(a: T, b: T): T | undefined {
    undefined as T | undefined
}

extension Arithmetic<T: Scalar> of T {
    checkedAdd(other: T): T | undefined {
        checkedAdd<T>(this, other)
    }
}

=== dir ===
interface Scalar {}
/// @generic.template symbol=Scalar parameters=(this: Scalar)
/// @type.symbol symbol=Scalar source="interface Scalar {}" type=Scalar
/// @definition.interface symbol=Scalar source="interface Scalar {}" template=(this: Scalar)
/// @definition.where symbol=Scalar source="interface Scalar {}" relation=satisfies left=this right=Scalar

function checkedAdd<T: Scalar>(a: T, b: T): T | undefined {
/// @generic.template symbol=checkedAdd parameters=(T#1: Scalar)
/// @type.symbol symbol=checkedAdd type=<T#1: Scalar>(T#1, T#1) => T#1 | undefined
/// @type.symbol symbol=checkedAdd.T source="T: Scalar" type=T#1
/// @resolution.name source=Scalar target=Scalar
/// @type.symbol symbol=checkedAdd.a source="a: T" type=T#1
/// @resolution.name source=T target=checkedAdd.T
/// @type.symbol symbol=checkedAdd.b source="b: T" type=T#1
/// @resolution.name source=T target=checkedAdd.T
/// @resolution.name source=T target=checkedAdd.T

    undefined
    /// @type.node source=undefined type=undefined

}

extension Arithmetic<T: Scalar> of T {
/// @generic.template symbol=Arithmetic parameters=(T#2: Scalar)
/// @definition.extension symbol=Arithmetic form=local target=T#2
/// @definition.method symbol=Arithmetic.checkedAdd slot=checkedAdd type=(this: T#2, T#2) => T#2 | undefined
/// @type.symbol symbol=Arithmetic.T source="T: Scalar" type=T#2
/// @resolution.name source=Scalar target=Scalar
/// @resolution.name source=T target=Arithmetic.T

    checkedAdd(other: T): T | undefined {
    /// @type.symbol symbol=Arithmetic.checkedAdd type=(this: T#2, T#2) => T#2 | undefined
    /// @type.symbol symbol=Arithmetic.checkedAdd.this type=T#2
    /// @type.symbol symbol=Arithmetic.checkedAdd.other source="other: T" type=T#2
    /// @resolution.name source=T target=Arithmetic.T
    /// @resolution.name source=T target=Arithmetic.T

        checkedAdd(this, other)
        /// @type.node source="checkedAdd(this, other)" type=T#2 | undefined
        /// @type.node source=checkedAdd type=(T#2, T#2) => T#2 | undefined
        /// @resolution.name source=checkedAdd target=checkedAdd
        /// @resolution.call source="checkedAdd(this, other)" parameters=(T#2, T#2) arguments=(provided(this) as T#2, provided(other) as T#2) return=T#2 | undefined kind=symbol target=checkedAdd instance=checkedAdd<T#2>
        /// @generic.instantiation id=checkedAdd<T#2> template=checkedAdd arguments=(T#2) owner=Arithmetic.checkedAdd
        /// @type.node source=this type=T#2
        /// @resolution.receiver source=this kind=this declaration=Arithmetic type=T#2
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @type.node source=other type=T#2
        /// @resolution.name source=other target=Arithmetic.checkedAdd.other
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=Arithmetic.checkedAdd.other

    }
}
"#,
        r#"
"#,
    );
}

/// An extension method on a generic struct forwards to an unbounded free generic function.
#[test]
fn test_extension_method_forwards_to_unbounded_free_generic_function() {
    let session = TestSession::single(
        r#"
import { Copy } from "tspp:memory";

function choose<T>(a: T, b: T): T {
    a
}

struct Pair<T> {
    value: T;
}

extension Forward<T: Copy> of Pair<T> {
    choose(other: Pair<T>): Pair<T> {
        choose(this, other)
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Copy } from "tspp:memory";

function choose<T>(a: T, b: T): T {
    a
}

struct Pair<out T> {
    value: T;
}

extension Forward<T: Copy> of Pair<T> {
    choose(other: Pair<T>): Pair<T> {
        choose<Pair<T>>(this, other)
    }
}

=== dir ===
import { Copy } from "tspp:memory";

function choose<T>(a: T, b: T): T {
/// @generic.template symbol=choose parameters=(T#1)
/// @type.symbol symbol=choose type=<T#1>(T#1, T#1) => T#1
/// @type.symbol symbol=choose.T source=T type=T#1
/// @type.symbol symbol=choose.a source="a: T" type=T#1
/// @resolution.name source=T target=choose.T
/// @type.symbol symbol=choose.b source="b: T" type=T#1
/// @resolution.name source=T target=choose.T
/// @resolution.name source=T target=choose.T

    a
    /// @type.node source=a type=T#1
    /// @resolution.name source=a target=choose.a
    /// @resolution.place source=a placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=a root=choose.a

}

struct Pair<T> {
/// @generic.template symbol=Pair parameters=(out T#2)
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=(out T#2)
/// @definition.field symbol=Pair.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Pair.T source=T type=T#2

    value: T;
    /// @type.symbol symbol=Pair.value source="value: T" type=T#2
    /// @resolution.name source=T target=Pair.T

}

extension Forward<T: Copy> of Pair<T> {
/// @generic.template symbol=Forward parameters=(T#3: Copy)
/// @definition.extension symbol=Forward form=local target=Pair<T#3>
/// @definition.method symbol=Forward.choose slot=choose type=<Forward.choose.'a>(this: &Forward.choose.'a readonly Pair<T#3>, Pair<T#3>) => Pair<T#3>
/// @type.symbol symbol=Forward.T source="T: Copy" type=T#3
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Pair target=Pair
/// @resolution.name source=T target=Forward.T

    choose(other: Pair<T>): Pair<T> {
    /// @generic.template symbol=Forward.choose parent=template#2 parameters=('a)
    /// @type.symbol symbol=Forward.choose type=<Forward.choose.'a>(this: &Forward.choose.'a readonly Pair<T#3>, Pair<T#3>) => Pair<T#3>
    /// @type.symbol symbol=Forward.choose.this type=&Forward.choose.'a readonly Pair<T#3>
    /// @type.symbol symbol=Forward.choose.other source="other: Pair<T>" type=Pair<T#3>
    /// @resolution.name source=Pair target=Pair
    /// @resolution.name source=T target=Forward.T
    /// @resolution.name source=Pair target=Pair
    /// @resolution.name source=T target=Forward.T

        choose(this, other)
        /// @type.node source="choose(this, other)" type=Pair<T#3>
        /// @type.node source=choose type=(Pair<T#3>, Pair<T#3>) => Pair<T#3>
        /// @resolution.name source=choose target=choose
        /// @resolution.call source="choose(this, other)" parameters=(Pair<T#3>, Pair<T#3>) arguments=(provided(this) as Pair<T#3>, provided(other) as Pair<T#3>) return=Pair<T#3> kind=symbol target=choose instance=choose<Pair<T#3>>
        /// @generic.instantiation id=choose<Pair<T#3>> template=choose arguments=(Pair<T#3>) owner=Forward.choose
        /// @type.node source=this type=&Forward.choose.'a readonly Pair<T#3>
        /// @resolution.receiver source=this kind=this declaration=Forward type=&Forward.choose.'a readonly Pair<T#3>
        /// @resolution.place source=this placement=Forward.choose.'a lifetime=Forward.choose.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @type.node source=other type=Pair<T#3>
        /// @resolution.name source=other target=Forward.choose.other
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=Forward.choose.other

    }
}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type '&'a readonly Pair<T>' is not assignable to the declared result type 'Pair<T>'"
/// @diagnostic.label line=13 column=37 span="{\n        choose(this, other)\n    }" line_source="choose(other: Pair<T>): Pair<T> {"
/// @diagnostic.error id=argument-not-assignable message="argument of type '&'a readonly Pair<T>' is not assignable to parameter of type 'Pair<T>'"
/// @diagnostic.label line=14 column=16 span="this" line_source="choose(this, other)"
/// @diagnostic.related line=14 column=9 span="choose(this, other)" line_source="choose(this, other)" message="in this call"
"#,
    );
}

/// A method call infers its receiver lifetime from the borrowed argument.
#[test]
fn test_infer_method_receiver_lifetime_from_borrowed_argument() {
    let session = TestSession::single(
        r#"
declare class Box<T> {
    get(&readonly this): &readonly T;
}

function read<T>(source: &readonly Box<T>): &readonly T {
    return source.get();
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<in out T> {
    get(&readonly this): &readonly T;
}

function read<T, 'a>(source: &'a readonly Box<T>): &'a readonly T {
    return source.get<T, 'a>();
}

=== dir ===
declare class Box<T> {
/// @generic.template symbol=Box parameters=(in out T#1)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out T#1)
/// @definition.method symbol=Box.get source="get(&readonly this): &readonly T" slot=get type=<Box.get.'a>(this: &Box.get.'a readonly Box<T#1>) => &Box.get.'a readonly T#1
/// @type.symbol symbol=Box.T source=T type=T#1

    get(&readonly this): &readonly T;
    /// @generic.template symbol=Box.get parent=template#0 parameters=('a)
    /// @type.symbol symbol=Box.get source="get(&readonly this): &readonly T" type=<Box.get.'a>(this: &Box.get.'a readonly Box<T#1>) => &Box.get.'a readonly T#1
    /// @type.symbol symbol=Box.get.this source="&readonly this" type=&Box.get.'a readonly Box<T#1>
    /// @resolution.name source=T target=Box.T

}

function read<T>(source: &readonly Box<T>): &readonly T {
/// @generic.template symbol=read parameters=(T#2, 'a)
/// @type.symbol symbol=read type=<T#2, read.'a>(&read.'a readonly Box<T#2>) => &read.'a readonly T#2
/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @type.symbol symbol=read.T source=T type=T#2
/// @type.symbol symbol=read.source source="source: &readonly Box<T>" type=&read.'a readonly Box<T#2>
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    return source.get();
    /// @type.node source=source type=&read.'a readonly Box<T#2>
    /// @type.node source=source.get type=<Box.get.'a>(this: &Box.get.'a readonly Box<T#2>) => &Box.get.'a readonly T#2
    /// @type.node source=source.get() type=&read.'a readonly T#2
    /// @resolution.name source=source target=read.source
    /// @resolution.member source=source.get receiver=&read.'a readonly Box<T#2> type=<Box.get.'a>(this: &Box.get.'a readonly Box<T#2>) => &Box.get.'a readonly T#2 kind=symbol target_receiver=&read.'a readonly Box<T#2> target=Box.get
    /// @resolution.call source=source.get() parameters=() return=&read.'a readonly T#2 regions=(read.'a) kind=symbol target=Box.get receiver=&read.'a readonly Box<T#2> instance=Box<T#2>.get<read.'a>
    /// @resolution.place source=source placement=read.'a lifetime=read.'a access="readonly"
    /// @resolution.access source=source root=read.source
    /// @generic.instantiation id="Box.get<T#2, read.'a>" template=Box.get arguments=(T#2, read.'a) owner=read
    /// @generic.instantiation id=Box.get<T#2> template=Box.get arguments=(T#2) owner=read
    /// @generic.instance id="Box.get<T#2, read.'a>" template=Box.get arguments=(T#2, read.'a)

}
"#,
    );
}
