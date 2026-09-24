use crate::tests::{DirRows, TestSession};

#[test]
fn test_receiver_projections_reduce_for_generic_extensions() {
    let session = TestSession::single(
        r#"
import { Numeric } from "destack:math";

interface Doubles {
    type Output;
    double(): this.Output;
}

struct Pair<T: Numeric> {
    x: T;
}

extension<T: Numeric> of Pair<T> implements Doubles {
    type Output = Pair<T>;

    double(): this.Output {
        Pair { x: this.x + this.x }
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
import { Numeric } from "destack:math";

interface Doubles {
    type Output;
    double(): this.Output;
}

struct Pair<out T: Numeric> {
    x: T;
}

extension<T: Numeric> of Pair<T> implements Doubles {
    type Output = Pair<T>;

    double(): Pair<T> {
        Pair<T> { x: this.x + this.x }
    }
}

=== dir ===
import { Numeric } from "destack:math";

interface Doubles {
/// @generic.template symbol=Doubles parameters=(this: Doubles)
/// @type.symbol symbol=Doubles type=Doubles
/// @definition.interface symbol=Doubles template=(this: Doubles)
/// @definition.where symbol=Doubles relation=satisfies left=this right=Doubles
/// @definition.associated.type symbol=Doubles.Output source="type Output" key=Output
/// @definition.method symbol=Doubles.double source="double(): this.Output" slot=double type=() => this.Output

    type Output;
    double(): this.Output;
    /// @type.symbol symbol=Doubles.double source="double(): this.Output" type=() => this.Output
    /// @resolution.name source=this.Output target=Doubles.Output

}

struct Pair<T: Numeric> {
/// @generic.template symbol=Pair parameters=(out T#1: Numeric)
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=(out T#1: Numeric)
/// @definition.field symbol=Pair.x source="x: T" key=x type=T#1
/// @type.symbol symbol=Pair.T source="T: Numeric" type=T#1
/// @resolution.name source=Numeric target=Numeric

    x: T;
    /// @type.symbol symbol=Pair.x source="x: T" type=T#1
    /// @resolution.name source=T target=Pair.T

}

extension<T: Numeric> of Pair<T> implements Doubles {
/// @generic.template symbol=<module>#2 parameters=(T#2: Numeric)
/// @generic.instance id=Pair<T#2> template=Pair arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Pair<T#2>
/// @definition.implements symbol=<module>#2 source=Doubles target=Doubles
/// @definition.associated.type symbol=Output source="type Output = Pair<T>" key=Output value=Pair<T#2>
/// @definition.method symbol=double slot=double type=<double.'a>(this: &double.'a readonly Pair<T#2>) => Pair<T#2>
/// @definition.conformance symbol=<module>#2 member=Output requirement=Doubles.Output
/// @definition.conformance symbol=<module>#2 member=double requirement=Doubles.double
/// @type.symbol symbol=T source="T: Numeric" type=T#2
/// @resolution.name source=Numeric target=Numeric
/// @resolution.name source=Pair target=Pair
/// @resolution.name source=T target=T
/// @resolution.name source=Doubles target=Doubles

    type Output = Pair<T>;
    /// @type.symbol symbol=Output source="type Output = Pair<T>" type=Pair<T#2>
    /// @resolution.name source=Pair target=Pair
    /// @resolution.name source=T target=T

    double(): this.Output {
    /// @generic.template symbol=double parent=template#2 parameters=('a)
    /// @type.symbol symbol=double type=<double.'a>(this: &double.'a readonly Pair<T#2>) => Pair<T#2>
    /// @type.symbol symbol=double.this type=&double.'a readonly Pair<T#2>
    /// @resolution.name source=this.Output target=Output

        Pair { x: this.x + this.x }
        /// @type.node source="Pair { x: this.x + this.x }" type=Pair<T#2>
        /// @resolution.name source=Pair target=Pair
        /// @type.node source="this.x + this.x" type=T#2
        /// @type.node source=this.x type=T#2
        /// @resolution.member source=this.x receiver=&double.'a readonly Pair<T#2> type=T#2 kind=field target_receiver=&double.'a readonly Pair<T#2> key=x target=Pair.x target_type=T#2
        /// @resolution.operator source="this.x + this.x" type=T#2 operator="+" kind=builtin operands=[this.x as T#2 families=(integer | float), this.x as T#2 families=(integer | float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&double.'a readonly Pair<T#2>
        /// @resolution.place source=this placement=double.'a lifetime=double.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=double.'a lifetime=double.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @type.node source=this.x type=T#2
        /// @resolution.member source=this.x receiver=&double.'a readonly Pair<T#2> type=T#2 kind=field target_receiver=&double.'a readonly Pair<T#2> key=x target=Pair.x target_type=T#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&double.'a readonly Pair<T#2>
        /// @resolution.place source=this placement=double.'a lifetime=double.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=double.'a lifetime=double.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]

    }
}
"#,
    );
}

/// An associated type projected through an interface handle reads the interface's default.
#[test]
fn test_project_an_associated_type_default_through_an_interface_handle() {
    let session = TestSession::single(
        r#"
interface Source<out T> {
    type Return = void;

    next(&this): T | this.Return;
}

struct Enumerated<I, out T> {
    inner: I;
}

extension<T, I: Source<T>> of Enumerated<I, T> implements Source<T> {
    type Return = I.Return;

    next(&this): T | I.Return {
        return this.inner.next();
    }
}

declare function source(): Source<int32>;

export function run(): void {
    let enumerated = Enumerated<Source<int32>, int32> { inner: source() };
    const next = enumerated.next();
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Source<out T> {
    type Return = void;

    next(&this): T | this.Return;
}

struct Enumerated<out I, out T> {
    inner: I;
}

extension<T, I: Source<T>> of Enumerated<I, T> implements Source<T> {
    type Return = I.Return;

    next(&this): T | I.Return {
        return this.inner.next<T, 'a>();
    }
}

declare function source(): Source<int32>;

export function run(): void {
    let enumerated: Enumerated<Source<int32>, int32> = Enumerated<Source<int32>, int32> {
        inner: source(),
    };
    const next: int32 | void = enumerated.next<int32, Source<int32>, "frame">();
}

=== dir ===
interface Source<out T> {
/// @generic.template symbol=Source parameters=(out T#1, this: Source<T#1>)
/// @type.symbol symbol=Source type=Source
/// @definition.interface symbol=Source template=(out T#1, this: Source<T#1>)
/// @definition.where symbol=Source relation=satisfies left=this right=Source<T#1>
/// @definition.associated.type symbol=Source.Return source="type Return = void" key=Return value=void
/// @definition.method symbol=Source.next source="next(&this): T | this.Return" slot=next type=<Source.next.'a>(this: &Source.next.'a this) => T#1 | this.Return
/// @type.symbol symbol=Source.T source="out T" type=T#1

    type Return = void;
    /// @type.symbol symbol=Source.Return source="type Return = void" type=void

    next(&this): T | this.Return;
    /// @generic.template symbol=Source.next parent=template#0 parameters=('a)
    /// @type.symbol symbol=Source.next source="next(&this): T | this.Return" type=<Source.next.'a>(this: &Source.next.'a this) => T#1 | this.Return
    /// @type.symbol symbol=Source.next.this source=&this type=&Source.next.'a this
    /// @resolution.name source=T target=Source.T
    /// @resolution.name source=this.Return target=Source.Return

}

struct Enumerated<I, out T> {
/// @generic.template symbol=Enumerated parameters=(out I#1, out T#2)
/// @type.symbol symbol=Enumerated type=Enumerated
/// @definition.struct symbol=Enumerated template=(out I#1, out T#2)
/// @definition.field symbol=Enumerated.inner source="inner: I" key=inner type=I#1
/// @type.symbol symbol=Enumerated.I source=I type=I#1
/// @type.symbol symbol=Enumerated.T source="out T" type=T#2

    inner: I;
    /// @type.symbol symbol=Enumerated.inner source="inner: I" type=I#1
    /// @resolution.name source=I target=Enumerated.I

}

extension<T, I: Source<T>> of Enumerated<I, T> implements Source<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3, I#2: Source<T#3>)
/// @generic.instance id="Enumerated<I#2, T#3>" template=Enumerated arguments=(I#2, T#3)
/// @generic.instance id=Source<T#3> template=Source arguments=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Enumerated<I#2, T#3>
/// @definition.implements symbol=<module>#2 source=Source<T> target=Source<T#3>
/// @definition.associated.type symbol=Return source="type Return = I.Return" key=Return value=I#2.Return
/// @definition.method symbol=next slot=next type=<next.'a>(this: &next.'a Enumerated<I#2, T#3>) => T#3 | I#2.Return
/// @definition.conformance symbol=<module>#2 member=Return requirement=Source.Return
/// @definition.conformance symbol=<module>#2 member=next requirement=Source.next
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=I source="I: Source<T>" type=I#2
/// @resolution.name source=Source target=Source
/// @resolution.name source=T target=T
/// @resolution.name source=Enumerated target=Enumerated
/// @resolution.name source=I target=I
/// @resolution.name source=T target=T
/// @resolution.name source=Source target=Source
/// @resolution.name source=T target=T

    type Return = I.Return;
    /// @type.symbol symbol=Return source="type Return = I.Return" type=I#2.Return
    /// @resolution.name source=I.Return target=I
    /// @resolution.path source=I.Return index=1 target=Source.Return

    next(&this): T | I.Return {
    /// @generic.template symbol=next parent=template#2 parameters=('a)
    /// @type.symbol symbol=next type=<next.'a>(this: &next.'a Enumerated<I#2, T#3>) => T#3 | I#2.Return
    /// @type.symbol symbol=next.this source=&this type=&next.'a Enumerated<I#2, T#3>
    /// @resolution.name source=T target=T
    /// @resolution.name source=I.Return target=I
    /// @resolution.path source=I.Return index=1 target=Source.Return

        return this.inner.next();
        /// @resolution.member source=this.inner receiver=&next.'a Enumerated<I#2, T#3> type=I#2 kind=field target_receiver=&next.'a Enumerated<I#2, T#3> key=inner target=Enumerated.inner target_type=I#2
        /// @resolution.member source=this.inner.next receiver=I#2 type=<Source.next.'a>(this: &Source.next.'a I#2) => T#3 | I#2.Return kind=symbol target_receiver=I#2 target=Source.next
        /// @resolution.call source=this.inner.next() parameters=() return=T#3 | I#2.Return regions=(next.'a) kind=symbol target=Source.next receiver=I#2 adjustments=(borrow(&next.'a I#2)) instance=Source<T#3>.next<next.'a>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&next.'a Enumerated<I#2, T#3>
        /// @resolution.place source=this placement=next.'a lifetime=next.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.inner placement=next.'a lifetime=next.'a access="mutable"
        /// @resolution.access source=this.inner root=this keys=[inner]
        /// @generic.instantiation id="Source.next<I#2, T#3, next.'a>" template=Source.next arguments=(T#3, next.'a) owner=next
        /// @generic.instantiation id=Source.next<T#3> template=Source.next arguments=(T#3) owner=next
        /// @generic.instance id="Source.next<I#2, T#3, next.'a>" template=Source.next arguments=(T#3, next.'a)

    }
}

declare function source(): Source<int32>;
/// @type.symbol symbol=source source="declare function source(): Source<int32>" type=() => Source<int32>
/// @generic.instance id=Source<int32> template=Source arguments=(int32)
/// @resolution.name source=Source target=Source

export function run(): void {
/// @type.symbol symbol=run type=() => void

    let enumerated = Enumerated<Source<int32>, int32> { inner: source() };
    /// @type.symbol symbol=run.enumerated source=enumerated type=Enumerated<Source<int32>, int32>
    /// @resolution.pattern source=enumerated kind=binding target=run.enumerated
    /// @generic.instance id="Enumerated<Source<int32>, int32>" template=Enumerated arguments=(Source<int32>, int32)
    /// @resolution.name source=Enumerated target=Enumerated
    /// @resolution.name source=Source target=Source
    /// @resolution.name source=source target=source
    /// @resolution.call source=source() parameters=() return=Source<int32> kind=symbol target=source

    const next = enumerated.next();
    /// @type.symbol symbol=run.next source=next type=int32 | void
    /// @resolution.pattern source=next kind=binding target=run.next
    /// @resolution.name source=enumerated target=run.enumerated
    /// @resolution.member source=enumerated.next receiver=Enumerated<Source<int32>, int32> type=<next.'a>(this: &next.'a Enumerated<Source<int32>, int32>) => int32 | void kind=symbol target_receiver=Enumerated<Source<int32>, int32> target=next
    /// @resolution.call source=enumerated.next() parameters=() return=int32 | void regions=("frame" & "local") kind=symbol target=next receiver=Enumerated<Source<int32>, int32> adjustments=(borrow(&'frame Enumerated<Source<int32>, int32>)) instance="Enumerated<Source<int32>, int32>.<extension#1>.next<\"frame\" & \"local\">"
    /// @resolution.place source=enumerated placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=enumerated root=run.enumerated
    /// @generic.instantiation id="next<int32, Source<int32>, \"frame\" & \"local\">" template=next arguments=(int32, Source<int32>, "frame" & "local")
    /// @generic.instantiation id="next<int32, Source<int32>>" template=next arguments=(int32, Source<int32>)
    /// @generic.instance id="next<int32, Source<int32>, \"bound0\" & \"local\">" template=next arguments=(int32, Source<int32>, "bound0" & "local")

}
"#,
    );
}

/// An associated type projected through a newtype interface handle reads the interface's default.
#[test]
fn test_project_an_associated_type_default_through_a_newtype_interface_handle() {
    let session = TestSession::single(
        r#"
newtype interface Source<out T> {
    type Return = void;

    next(&this): T | this.Return;

    enumerate(this): Enumerated<this, T> {
        todo("enumerate")
    }
}

struct Enumerated<I, out T> {
    inner: I;
}

extension<T, I: Source<T>> of Enumerated<I, T> implements Source<T> {
    type Return = I.Return;

    next(&this): T | I.Return {
        return this.inner.next();
    }
}

declare function source(): Source<int32>;

export function run(): void {
    const next = source().enumerate().next();
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Source<out T> {
    type Return = void;

    next(&this): T | this.Return;

    enumerate(this): Enumerated<this, T> {
        todo("enumerate" as string | undefined)
    }
}

struct Enumerated<out I, out T> {
    inner: I;
}

extension<T, I: Source<T>> of Enumerated<I, T> implements Source<T> {
    type Return = I.Return;

    next(&this): T | I.Return {
        return this.inner.next<T, 'a>();
    }
}

declare function source(): Source<int32>;

export function run(): void {
    const next: int32 | void = source().enumerate<int32>().next<int32, Source<int32>, "frame">();
}

=== dir ===
newtype interface Source<out T> {
/// @generic.template symbol=Source parameters=(out T#1, this: Source<T#1>)
/// @type.symbol symbol=Source type=Source
/// @definition.interface symbol=Source template=(out T#1, this: Source<T#1>) nominal=true
/// @definition.where symbol=Source relation=satisfies left=this right=Source<T#1>
/// @definition.associated.type symbol=Source.Return source="type Return = void" key=Return value=void
/// @definition.method symbol=Source.enumerate slot=enumerate type=(this: this) => Enumerated<this, T#1>
/// @definition.method symbol=Source.next source="next(&this): T | this.Return" slot=next type=<Source.next.'a>(this: &Source.next.'a this) => T#1 | this.Return
/// @type.symbol symbol=Source.T source="out T" type=T#1

    type Return = void;
    /// @type.symbol symbol=Source.Return source="type Return = void" type=void

    next(&this): T | this.Return;
    /// @generic.template symbol=Source.next parent=template#0 parameters=('a)
    /// @type.symbol symbol=Source.next source="next(&this): T | this.Return" type=<Source.next.'a>(this: &Source.next.'a this) => T#1 | this.Return
    /// @type.symbol symbol=Source.next.this source=&this type=&Source.next.'a this
    /// @resolution.name source=T target=Source.T
    /// @resolution.name source=this.Return target=Source.Return

    enumerate(this): Enumerated<this, T> {
    /// @type.symbol symbol=Source.enumerate type=(this: this) => Enumerated<this, T#1>
    /// @generic.instance id="Enumerated<this, T#1>" template=Enumerated arguments=(this, T#1)
    /// @type.symbol symbol=Source.enumerate.this source=this type=this
    /// @resolution.name source=Enumerated target=Enumerated
    /// @resolution.name source=T target=Source.T

        todo("enumerate")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"enumerate\")" parameters=(string | undefined) arguments=(provided("enumerate") as string | undefined) return=never kind=symbol target=todo

    }
}

struct Enumerated<I, out T> {
/// @generic.template symbol=Enumerated parameters=(out I#1, out T#2)
/// @type.symbol symbol=Enumerated type=Enumerated
/// @definition.struct symbol=Enumerated template=(out I#1, out T#2)
/// @definition.field symbol=Enumerated.inner source="inner: I" key=inner type=I#1
/// @type.symbol symbol=Enumerated.I source=I type=I#1
/// @type.symbol symbol=Enumerated.T source="out T" type=T#2

    inner: I;
    /// @type.symbol symbol=Enumerated.inner source="inner: I" type=I#1
    /// @resolution.name source=I target=Enumerated.I

}

extension<T, I: Source<T>> of Enumerated<I, T> implements Source<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3, I#2: Source<T#3>)
/// @generic.instance id="Enumerated<I#2, T#3>" template=Enumerated arguments=(I#2, T#3)
/// @generic.instance id=Source<T#3> template=Source arguments=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Enumerated<I#2, T#3>
/// @definition.implements symbol=<module>#2 source=Source<T> target=Source<T#3>
/// @definition.associated.type symbol=Return source="type Return = I.Return" key=Return value=I#2.Return
/// @definition.method symbol=next slot=next type=<next.'a>(this: &next.'a Enumerated<I#2, T#3>) => T#3 | I#2.Return
/// @definition.conformance symbol=<module>#2 member=Return requirement=Source.Return
/// @definition.conformance symbol=<module>#2 member=Source.enumerate requirement=Source.enumerate
/// @definition.conformance symbol=<module>#2 member=next requirement=Source.next
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=I source="I: Source<T>" type=I#2
/// @resolution.name source=Source target=Source
/// @resolution.name source=T target=T
/// @resolution.name source=Enumerated target=Enumerated
/// @resolution.name source=I target=I
/// @resolution.name source=T target=T
/// @resolution.name source=Source target=Source
/// @resolution.name source=T target=T

    type Return = I.Return;
    /// @type.symbol symbol=Return source="type Return = I.Return" type=I#2.Return
    /// @resolution.name source=I.Return target=I
    /// @resolution.path source=I.Return index=1 target=Source.Return

    next(&this): T | I.Return {
    /// @generic.template symbol=next parent=template#2 parameters=('a)
    /// @type.symbol symbol=next type=<next.'a>(this: &next.'a Enumerated<I#2, T#3>) => T#3 | I#2.Return
    /// @type.symbol symbol=next.this source=&this type=&next.'a Enumerated<I#2, T#3>
    /// @resolution.name source=T target=T
    /// @resolution.name source=I.Return target=I
    /// @resolution.path source=I.Return index=1 target=Source.Return

        return this.inner.next();
        /// @resolution.member source=this.inner receiver=&next.'a Enumerated<I#2, T#3> type=I#2 kind=field target_receiver=&next.'a Enumerated<I#2, T#3> key=inner target=Enumerated.inner target_type=I#2
        /// @resolution.member source=this.inner.next receiver=I#2 type=<Source.next.'a>(this: &Source.next.'a I#2) => T#3 | I#2.Return kind=symbol target_receiver=I#2 target=Source.next
        /// @resolution.call source=this.inner.next() parameters=() return=T#3 | I#2.Return regions=(next.'a) kind=symbol target=Source.next receiver=I#2 adjustments=(borrow(&next.'a I#2)) instance=Source<T#3>.next<next.'a>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&next.'a Enumerated<I#2, T#3>
        /// @resolution.place source=this placement=next.'a lifetime=next.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.inner placement=next.'a lifetime=next.'a access="mutable"
        /// @resolution.access source=this.inner root=this keys=[inner]
        /// @generic.instantiation id="Source.next<I#2, T#3, next.'a>" template=Source.next arguments=(T#3, next.'a) owner=next
        /// @generic.instantiation id=Source.next<T#3> template=Source.next arguments=(T#3) owner=next
        /// @generic.instance id="Source.next<I#2, T#3, next.'a>" template=Source.next arguments=(T#3, next.'a)

    }
}

declare function source(): Source<int32>;
/// @type.symbol symbol=source source="declare function source(): Source<int32>" type=() => Source<int32>
/// @generic.instance id=Source<int32> template=Source arguments=(int32)
/// @resolution.name source=Source target=Source

export function run(): void {
/// @type.symbol symbol=run type=() => void

    const next = source().enumerate().next();
    /// @type.symbol symbol=run.next source=next type=int32 | void
    /// @resolution.pattern source=next kind=binding target=run.next
    /// @resolution.name source=source target=source
    /// @resolution.member source=source().enumerate receiver=Source<int32> type=(this: Source<int32>) => Enumerated<Source<int32>, int32> kind=symbol target_receiver=Source<int32> dispatch=dynamic constraint=Source<int32> target=Source.enumerate
    /// @resolution.member source=source().enumerate().next receiver=Enumerated<Source<int32>, int32> type=<next.'a>(this: &next.'a Enumerated<Source<int32>, int32>) => int32 | void kind=symbol target_receiver=Enumerated<Source<int32>, int32> target=next
    /// @resolution.call source=source() parameters=() return=Source<int32> kind=symbol target=source
    /// @resolution.call source=source().enumerate() parameters=() return=Enumerated<Source<int32>, int32> kind=dynamic target=Source.enumerate receiver=Source<int32> constraint=Source<int32> generic_arguments=(int32)
    /// @resolution.call source=source().enumerate().next() parameters=() return=int32 | void regions=("frame" & "local") kind=symbol target=next receiver=Enumerated<Source<int32>, int32> adjustments=(borrow(&'frame Enumerated<Source<int32>, int32>)) instance="Enumerated<Source<int32>, int32>.<extension#1>.next<\"frame\" & \"local\">"
    /// @generic.instantiation id="next<int32, Source<int32>, \"frame\" & \"local\">" template=next arguments=(int32, Source<int32>, "frame" & "local")
    /// @generic.instantiation id="next<int32, Source<int32>>" template=next arguments=(int32, Source<int32>)
    /// @generic.instantiation id=Source.enumerate<int32> template=Source.enumerate arguments=(int32)
    /// @generic.instance id="Enumerated<Source<int32>, int32>" template=Enumerated arguments=(Source<int32>, int32)
    /// @generic.instance id="next<int32, Source<int32>, \"bound0\" & \"local\">" template=next arguments=(int32, Source<int32>, "bound0" & "local")

}
"#,
    );
}
