use crate::tests::{DirRows, TestSession};

#[test]
fn test_associated_type_uses_owner_generic_argument() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    type Item = T;
    value: T;
}

declare const value: Box<string>.Item;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
struct Box<out T> {
    type Item = T;
    value: T;
}

declare const value: string;

=== dir ===
struct Box<T> {
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T)
/// @definition.associated.type symbol=Box.Item source="type Item = T" key=Item value=T
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @type.symbol symbol=Box.T source=T type=T

    type Item = T;
    /// @type.symbol symbol=Box.Item source="type Item = T" type=T
    /// @resolution.name source=T target=Box.T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

declare const value: Box<string>.Item;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box<string>.Item target=Box.Item
/// @generic.instance id=Box<string> template=Box arguments=(string)
"#,
    );
}

#[test]
fn test_associated_type_used_as_value_reports_error() {
    let session = TestSession::single(
        r#"
class Packet {
    type Size = uint32;
}

const size = Packet.Size;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Packet {
    type Size = uint32;
}

const size: uint32 = Packet.Size;

=== dir ===
class Packet {
/// @type.symbol symbol=Packet type=typeof Packet
/// @definition.class symbol=Packet
/// @definition.associated.type symbol=Packet.Size source="type Size = uint32" key=Size value=uint32

    type Size = uint32;
    /// @type.symbol symbol=Packet.Size source="type Size = uint32" type=uint32

}

const size = Packet.Size;
/// @type.symbol symbol=size source=size type=uint32
/// @resolution.pattern source=size kind=binding target=size
/// @resolution.name source=Packet target=Packet
/// @resolution.member source=Packet.Size receiver=typeof Packet type=uint32 kind=symbol target_receiver=typeof Packet target=Packet.Size
"#,
        r#"
"#,
    );
}

#[test]
fn test_associated_type_refinement_flows_through_constraint() {
    let session = TestSession::single(
        r#"
interface Iterator {
    type Item;

    next(): this.Item;
}

function nextByte<I: Iterator<type Item = uint8>>(iter: I): uint8 {
    return iter.next();
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Iterator {
    type Item;

    next(): this.Item;
}

function nextByte<I: Iterator<type Item = uint8>>(iter: I): uint8 {
    return iter.next();
}

=== dir ===
interface Iterator {
/// @generic.template symbol=Iterator parameters=(this: Iterator)
/// @type.symbol symbol=Iterator type=Iterator
/// @definition.interface symbol=Iterator template=(this: Iterator)
/// @definition.where symbol=Iterator relation=satisfies left=this right=Iterator
/// @definition.associated.type symbol=Iterator.Item source="type Item" key=Item
/// @definition.method symbol=Iterator.next source="next(): this.Item" slot=next type=() => this.Item

    type Item;

    next(): this.Item;
    /// @type.symbol symbol=Iterator.next source="next(): this.Item" type=() => this.Item
    /// @resolution.name source=this.Item target=Iterator.Item

}

function nextByte<I: Iterator<type Item = uint8>>(iter: I): uint8 {
/// @generic.template symbol=nextByte parameters=(I: Iterator<type Item = uint8>)
/// @type.symbol symbol=nextByte type=<I: Iterator<type Item = uint8>>(I) => uint8
/// @type.symbol symbol=nextByte.I source="I: Iterator<type Item = uint8>" type=I
/// @resolution.name source=Iterator target=Iterator
/// @resolution.name source="type Item = uint8" target=Iterator.Item
/// @type.symbol symbol=nextByte.iter source="iter: I" type=I
/// @resolution.name source=I target=nextByte.I

    return iter.next();
    /// @resolution.name source=iter target=nextByte.iter
    /// @resolution.member source=iter.next receiver=I type=() => uint8 kind=symbol target_receiver=I target=Iterator.next
    /// @resolution.call source=iter.next() parameters=() return=uint8 kind=symbol target=Iterator.next receiver=I
    /// @resolution.place source=iter placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=iter root=nextByte.iter

}
"#,
    );
}

#[test]
fn test_direct_implementation_projects_its_associated_type() {
    let session = TestSession::single(
        r#"
interface Producing {
    type Output;

    produce(): this.Output;
}

class Factory implements Producing {
    type Output = int32;

    produce(): this.Output {
        return 7;
    }
}

type Made<F: Producing> = F.Output;

declare const made: Made<Factory>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Producing {
    type Output;

    produce(): this.Output;
}

class Factory implements Producing {
    type Output = int32;

    produce(): int32 {
        return 7;
    }
}

type Made<F: Producing> = F.Output;

declare const made: Made<Factory>;

=== dir ===
interface Producing {
/// @generic.template symbol=Producing parameters=(this: Producing)
/// @type.symbol symbol=Producing type=Producing
/// @definition.interface symbol=Producing template=(this: Producing)
/// @definition.where symbol=Producing relation=satisfies left=this right=Producing
/// @definition.associated.type symbol=Producing.Output source="type Output" key=Output
/// @definition.method symbol=Producing.produce source="produce(): this.Output" slot=produce type=() => this.Output

    type Output;

    produce(): this.Output;
    /// @type.symbol symbol=Producing.produce source="produce(): this.Output" type=() => this.Output
    /// @resolution.name source=this.Output target=Producing.Output

}

class Factory implements Producing {
/// @type.symbol symbol=Factory type=typeof Factory
/// @definition.class symbol=Factory
/// @definition.where symbol=Factory source=Producing relation=satisfies left=this right=Producing
/// @definition.implements symbol=Factory source=Producing target=Producing
/// @definition.associated.type symbol=Factory.Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=Factory.produce slot=produce type=(this: Factory) => int32
/// @definition.conformance symbol=Factory member=Factory.Output requirement=Producing.Output
/// @definition.conformance symbol=Factory member=Factory.produce requirement=Producing.produce
/// @resolution.name source=Producing target=Producing

    type Output = int32;
    /// @type.symbol symbol=Factory.Output source="type Output = int32" type=int32

    produce(): this.Output {
    /// @type.symbol symbol=Factory.produce type=(this: Factory) => int32
    /// @type.symbol symbol=Factory.produce.this type=Factory
    /// @resolution.name source=this.Output target=Producing.Output

        return 7;
    }
}

type Made<F: Producing> = F.Output;
/// @generic.template symbol=Made parameters=(F: Producing)
/// @type.symbol symbol=Made source="type Made<F: Producing> = F.Output" type=F.Output
/// @definition.type symbol=Made source="type Made<F: Producing> = F.Output" template=(F: Producing) value=F.Output
/// @type.symbol symbol=Made.F source="F: Producing" type=F
/// @resolution.name source=Producing target=Producing
/// @resolution.name source=F.Output target=Made.F
/// @resolution.path source=F.Output index=1 target=Producing.Output

declare const made: Made<Factory>;
/// @type.symbol symbol=made source=made type=Made<Factory>
/// @resolution.pattern source=made kind=binding target=made
/// @resolution.name source=Made target=Made
/// @resolution.name source=Factory target=Factory
"#,
    );
}

#[test]
fn test_associated_type_default_stays_conformance_only() {
    // a defaulted associated type fills conforming implementers, and a rigid
    //  bound keeps the projection since an implementer may override the default
    let session = TestSession::single(
        r#"
interface Iterator {
    type Item = uint8;

    next(): this.Item;
}

function nextDefault<I: Iterator>(iter: I): uint8 {
    return iter.next();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Iterator {
    type Item = uint8;

    next(): this.Item;
}

function nextDefault<I: Iterator>(iter: I): uint8 {
    return iter.next();
}

=== dir ===
interface Iterator {
/// @generic.template symbol=Iterator parameters=(this: Iterator)
/// @type.symbol symbol=Iterator type=Iterator
/// @definition.interface symbol=Iterator template=(this: Iterator)
/// @definition.where symbol=Iterator relation=satisfies left=this right=Iterator
/// @definition.associated.type symbol=Iterator.Item source="type Item = uint8" key=Item value=uint8
/// @definition.method symbol=Iterator.next source="next(): this.Item" slot=next type=() => this.Item

    type Item = uint8;
    /// @type.symbol symbol=Iterator.Item source="type Item = uint8" type=uint8

    next(): this.Item;
    /// @type.symbol symbol=Iterator.next source="next(): this.Item" type=() => this.Item
    /// @resolution.name source=this.Item target=Iterator.Item

}

function nextDefault<I: Iterator>(iter: I): uint8 {
/// @generic.template symbol=nextDefault parameters=(I: Iterator)
/// @type.symbol symbol=nextDefault type=<I: Iterator>(I) => uint8
/// @type.symbol symbol=nextDefault.I source="I: Iterator" type=I
/// @resolution.name source=Iterator target=Iterator
/// @type.symbol symbol=nextDefault.iter source="iter: I" type=I
/// @resolution.name source=I target=nextDefault.I

    return iter.next();
    /// @resolution.name source=iter target=nextDefault.iter
    /// @resolution.member source=iter.next receiver=I type=() => I.Item kind=symbol target_receiver=I target=Iterator.next
    /// @resolution.call source=iter.next() parameters=() return=I.Item kind=symbol target=Iterator.next receiver=I
    /// @resolution.place source=iter placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=iter root=nextDefault.iter

}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'I.Item' is not assignable to the declared result type 'uint8'"
/// @diagnostic.label line=9 column=12 span="iter.next()" line_source="return iter.next();"
"#,
    );
}

#[test]
fn test_member_path_segments_resolve_through_an_imported_base() {
    let session = TestSession::builder()
        .module(
            "geometry.ds",
            r#"
export struct Slot {
    type Value = int32;
}

export struct Grid {
    type Cell = Slot;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Grid } from "./geometry.ds";

declare const value: Grid.Cell.Value;
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
import { Grid } from "./geometry.ds";

declare const value: int32;

=== dir ===
import { Grid } from "./geometry.ds";

declare const value: Grid.Cell.Value;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Grid.Cell.Value target=geometry.Grid
/// @resolution.path source=Grid.Cell.Value index=1 target=geometry.Grid.Cell
/// @resolution.path source=Grid.Cell.Value index=2 target=geometry.Slot.Value
"#,
    );
}

#[test]
fn test_resolve_an_implemented_associated_type_through_the_class() {
    let session = TestSession::single(
        r#"
interface Envelope<T: string> {
    type Label<U: string> = `${T}:${U}`;
}

class Message<T: string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Envelope<in out T: string> {
    type Label<U: string> = `${T}:${U}`;
}

class Message<in out T: string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;

=== dir ===
interface Envelope<T: string> {
/// @generic.template symbol=Envelope parameters=(in out T#1: string, this: Envelope<T#1>)
/// @type.symbol symbol=Envelope type=Envelope
/// @definition.interface symbol=Envelope template=(in out T#1: string, this: Envelope<T#1>)
/// @definition.where symbol=Envelope relation=satisfies left=this right=Envelope<T#1>
/// @definition.associated.type symbol=Envelope.Label source="type Label<U: string> = `${T}:${U}`" key=Label value=`${T#1}:${U}`
/// @type.symbol symbol=Envelope.T source="T: string" type=T#1

    type Label<U: string> = `${T}:${U}`;
    /// @generic.template symbol=Envelope.Label parent=template#0 parameters=(U: string)
    /// @type.symbol symbol=Envelope.Label source="type Label<U: string> = `${T}:${U}`" type=`${T#1}:${U}`
    /// @type.symbol symbol=Envelope.Label.U source="U: string" type=U
    /// @resolution.name source=T target=Envelope.T
    /// @resolution.name source=U target=Envelope.Label.U

}

class Message<T: string> implements Envelope<T> {}
/// @generic.template symbol=Message parameters=(in out T#2: string)
/// @type.symbol symbol=Message source="class Message<T: string> implements Envelope<T> {}" type=typeof Message
/// @generic.instance id=Envelope<T#2> template=Envelope arguments=(T#2)
/// @definition.class symbol=Message source="class Message<T: string> implements Envelope<T> {}" template=(in out T#2: string)
/// @definition.where symbol=Message source=Envelope<T> relation=satisfies left=this right=Envelope<T#2>
/// @definition.implements symbol=Message source=Envelope<T> target=Envelope<T#2>
/// @definition.conformance symbol=Message member=Envelope.Label requirement=Envelope.Label
/// @type.symbol symbol=Message.T source="T: string" type=T#2
/// @resolution.name source=Envelope target=Envelope
/// @resolution.name source=T target=Message.T

type EventLabel = Message<"orders">.Label<"created">;
/// @type.symbol symbol=EventLabel source="type EventLabel = Message<\"orders\">.Label<\"created\">" type="orders:created"
/// @definition.type symbol=EventLabel source="type EventLabel = Message<\"orders\">.Label<\"created\">" value=Message<"orders">.Label<"created">
/// @resolution.name source="Message<\"orders\">.Label<\"created\">" target=Envelope.Label
/// @resolution.name source=Message target=Message
/// @generic.instance id="Message<\"orders\">" template=Message arguments=("orders")
"#,
    );
}

#[test]
fn test_resolve_an_imported_implemented_associated_type() {
    let session = TestSession::builder()
        .module(
            "envelope.ds",
            r#"
export interface Envelope<T: string> {
    type Label<U: string> = `${T}:${U}`;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Envelope } from "./envelope.ds";

class Message<T: string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Envelope } from "./envelope.ds";

class Message<in out T: string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;

=== dir ===
import { Envelope } from "./envelope.ds";

class Message<T: string> implements Envelope<T> {}
/// @generic.template symbol=Message parameters=(in out T: string)
/// @type.symbol symbol=Message source="class Message<T: string> implements Envelope<T> {}" type=typeof Message
/// @generic.instance id=envelope.Envelope<T> template=envelope.Envelope arguments=(T)
/// @definition.class symbol=Message source="class Message<T: string> implements Envelope<T> {}" template=(in out T: string)
/// @definition.where symbol=Message source=Envelope<T> relation=satisfies left=this right=envelope.Envelope<T>
/// @definition.implements symbol=Message source=Envelope<T> target=envelope.Envelope<T>
/// @definition.conformance symbol=Message member=envelope.Envelope.Label requirement=envelope.Envelope.Label
/// @type.symbol symbol=Message.T source="T: string" type=T
/// @resolution.name source=Envelope target=envelope.Envelope
/// @resolution.name source=T target=Message.T

type EventLabel = Message<"orders">.Label<"created">;
/// @type.symbol symbol=EventLabel source="type EventLabel = Message<\"orders\">.Label<\"created\">" type="orders:created"
/// @definition.type symbol=EventLabel source="type EventLabel = Message<\"orders\">.Label<\"created\">" value=Message<"orders">.Label<"created">
/// @resolution.name source="Message<\"orders\">.Label<\"created\">" target=envelope.Envelope.Label
/// @resolution.name source=Message target=Message
/// @generic.instance id="Message<\"orders\">" template=Message arguments=("orders")
"#,
    );
}

/// Store a numeric literal into a projection of a parameter the call infers later.
#[test]
fn test_store_a_numeric_literal_into_a_projection_of_an_inferred_parameter() {
    let session = TestSession::single(
        r#"
interface Producing {
    type Output;

    produce(): this.Output;
}

class Factory implements Producing {
    type Output = int32;

    produce(): this.Output {
        return 7;
    }
}

function take<F: Producing>(initial: F.Output, factory: F): F.Output {
    return initial;
}

const taken = take(0, new Factory());
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Producing {
    type Output;

    produce(): this.Output;
}

class Factory implements Producing {
    type Output = int32;

    produce(): int32 {
        return 7;
    }
}

function take<F: Producing>(initial: F.Output, factory: F): F.Output {
    return initial;
}

const taken: int32 = take<Factory>(0, new Factory());

=== dir ===
interface Producing {
/// @generic.template symbol=Producing parameters=(this: Producing)
/// @type.symbol symbol=Producing type=Producing
/// @definition.interface symbol=Producing template=(this: Producing)
/// @definition.where symbol=Producing relation=satisfies left=this right=Producing
/// @definition.associated.type symbol=Producing.Output source="type Output" key=Output
/// @definition.method symbol=Producing.produce source="produce(): this.Output" slot=produce type=() => this.Output

    type Output;

    produce(): this.Output;
    /// @type.symbol symbol=Producing.produce source="produce(): this.Output" type=() => this.Output
    /// @resolution.name source=this.Output target=Producing.Output

}

class Factory implements Producing {
/// @type.symbol symbol=Factory type=typeof Factory
/// @definition.class symbol=Factory
/// @definition.where symbol=Factory source=Producing relation=satisfies left=this right=Producing
/// @definition.implements symbol=Factory source=Producing target=Producing
/// @definition.associated.type symbol=Factory.Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=Factory.produce slot=produce type=(this: Factory) => int32
/// @definition.conformance symbol=Factory member=Factory.Output requirement=Producing.Output
/// @definition.conformance symbol=Factory member=Factory.produce requirement=Producing.produce
/// @resolution.name source=Producing target=Producing

    type Output = int32;
    /// @type.symbol symbol=Factory.Output source="type Output = int32" type=int32

    produce(): this.Output {
    /// @type.symbol symbol=Factory.produce type=(this: Factory) => int32
    /// @type.symbol symbol=Factory.produce.this type=Factory
    /// @resolution.name source=this.Output target=Producing.Output

        return 7;
    }
}

function take<F: Producing>(initial: F.Output, factory: F): F.Output {
/// @generic.template symbol=take parameters=(F: Producing)
/// @type.symbol symbol=take type=<F: Producing>(F.Output, F) => F.Output
/// @type.symbol symbol=take.F source="F: Producing" type=F
/// @resolution.name source=Producing target=Producing
/// @type.symbol symbol=take.initial source="initial: F.Output" type=F.Output
/// @resolution.name source=F.Output target=take.F
/// @resolution.path source=F.Output index=1 target=Producing.Output
/// @type.symbol symbol=take.factory source="factory: F" type=F
/// @resolution.name source=F target=take.F
/// @resolution.name source=F.Output target=take.F
/// @resolution.path source=F.Output index=1 target=Producing.Output

    return initial;
    /// @resolution.name source=initial target=take.initial
    /// @resolution.place source=initial placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=initial root=take.initial

}

const taken = take(0, new Factory());
/// @type.symbol symbol=taken source=taken type=int32
/// @resolution.pattern source=taken kind=binding target=taken
/// @resolution.name source=take target=take
/// @resolution.call source="take(0, new Factory())" parameters=(int32, Factory) arguments=(provided(0) as int32, provided(new Factory()) as Factory) return=int32 kind=symbol target=take instance=take<Factory>
/// @generic.instantiation id=take<Factory> template=take arguments=(Factory)
/// @generic.instance id=take<Factory> template=take arguments=(Factory)
/// @resolution.construct source="new Factory()" parameters=() return=Factory kind=class target=Factory constructor=default
/// @resolution.name source=Factory target=Factory
"#,
    );
}

#[test]
fn test_reject_an_unknown_refinement_name() {
    let session = TestSession::single(
        r#"
interface Container {
    type Item;
}

type Bad = Container<type Wrong = string>;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Container {
    type Item;
}

type Bad = Container<type Wrong = string>;

=== dir ===
interface Container {
/// @generic.template symbol=Container parameters=(this: Container)
/// @type.symbol symbol=Container type=Container
/// @definition.interface symbol=Container template=(this: Container)
/// @definition.where symbol=Container relation=satisfies left=this right=Container
/// @definition.associated.type symbol=Container.Item source="type Item" key=Item

    type Item;
}

type Bad = Container<type Wrong = string>;
/// @type.symbol symbol=Bad source="type Bad = Container<type Wrong = string>" type=Container<type Wrong = string>
/// @definition.type symbol=Bad source="type Bad = Container<type Wrong = string>" value=Container<type Wrong = string>
/// @resolution.name source=Container target=Container
"#, r#"
/// @diagnostic.error id=missing-member message="member 'Wrong' does not exist on type 'Container'"
/// @diagnostic.label line=6 column=27 span="Wrong" line_source="type Bad = Container<type Wrong = string>;"
"#);
}
