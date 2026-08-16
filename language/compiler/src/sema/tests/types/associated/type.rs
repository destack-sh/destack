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
/// @type.symbol symbol=Packet type=Packet
/// @definition.class symbol=Packet
/// @definition.associated.type symbol=Packet.Size source="type Size = uint32" key=Size value=uint32

    type Size = uint32;
    /// @type.symbol symbol=Packet.Size source="type Size = uint32" type=uint32

}

const size = Packet.Size;
/// @type.symbol symbol=size source=size type=uint32
/// @resolution.pattern source=size kind=binding target=size
/// @resolution.name source=Packet target=Packet
/// @resolution.member source=Packet.Size receiver=Packet type=uint32 kind=symbol target_receiver=Packet target=Packet.Size
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
/// @type.symbol symbol=Iterator type=Iterator
/// @definition.interface symbol=Iterator
/// @definition.associated.type symbol=Iterator.Item source="type Item" key=Item
/// @definition.method symbol=Iterator.next source="next(): this.Item" slot=next type=(this: Iterator) => Iterator.Item

    type Item;

    next(): this.Item;
    /// @type.symbol symbol=Iterator.next source="next(): this.Item" type=(this: Iterator) => Iterator.Item

}

function nextByte<I: Iterator<type Item = uint8>>(iter: I): uint8 {
/// @generic.template symbol=nextByte parameters=(I: Iterator<type Item = uint8>)
/// @type.symbol symbol=nextByte type=<I: Iterator<type Item = uint8>>(I) => uint8
/// @type.symbol symbol=nextByte.I source="I: Iterator<type Item = uint8>" type=I
/// @resolution.name source=Iterator target=Iterator
/// @type.symbol symbol=nextByte.iter source="iter: I" type=I
/// @resolution.name source=I target=nextByte.I

    return iter.next();
    /// @resolution.name source=iter target=nextByte.iter
    /// @resolution.member source=iter.next receiver=I type=(this: I) => I.Item kind=symbol target_receiver=I target=Iterator.next
    /// @resolution.call source=iter.next() parameters=() return=I.Item kind=symbol target=Iterator.next receiver=I
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

    produce(): this.Output {
        return 7;
    }
}

type Made<F: Producing> = F.Output;

declare const made: int32;

=== dir ===
interface Producing {
/// @type.symbol symbol=Producing type=Producing
/// @definition.interface symbol=Producing
/// @definition.associated.type symbol=Producing.Output source="type Output" key=Output
/// @definition.method symbol=Producing.produce source="produce(): this.Output" slot=produce type=(this: Producing) => Producing.Output

    type Output;

    produce(): this.Output;
    /// @type.symbol symbol=Producing.produce source="produce(): this.Output" type=(this: Producing) => Producing.Output

}

class Factory implements Producing {
/// @type.symbol symbol=Factory type=Factory
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
/// @type.symbol symbol=made source=made type=int32
/// @resolution.pattern source=made kind=binding target=made
/// @resolution.name source=Made target=Made
/// @resolution.name source=Factory target=Factory
"#,
    );
}

#[test]
fn test_associated_type_default_stays_conformance_only() {
    // a defaulted associated type fills conforming implementers, but an
    // implementer may override it, so rigid bounds never observe the default
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
/// @type.symbol symbol=Iterator type=Iterator
/// @definition.interface symbol=Iterator
/// @definition.associated.type symbol=Iterator.Item source="type Item = uint8" key=Item value=uint8
/// @definition.method symbol=Iterator.next source="next(): this.Item" slot=next type=(this: this) => this.Item

    type Item = uint8;
    /// @type.symbol symbol=Iterator.Item source="type Item = uint8" type=uint8

    next(): this.Item;
    /// @type.symbol symbol=Iterator.next source="next(): this.Item" type=(this: this) => this.Item

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
    /// @resolution.member source=iter.next receiver=I type=(this: I) => I.Item kind=symbol target_receiver=I target=Iterator.next
    /// @resolution.call source=iter.next() parameters=() return=I.Item kind=symbol target=Iterator.next receiver=I
    /// @resolution.place source=iter placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=iter root=nextDefault.iter

}
"#,
        r#"
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
/// @generic.template symbol=Envelope parameters=(in out T#1: string)
/// @type.symbol symbol=Envelope type=Envelope
/// @definition.interface symbol=Envelope template=(in out T#1: string)
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
/// @type.symbol symbol=Message source="class Message<T: string> implements Envelope<T> {}" type=Message
/// @definition.class symbol=Message source="class Message<T: string> implements Envelope<T> {}" template=(in out T#2: string)
/// @definition.where symbol=Message source=Envelope<T> relation=satisfies left=this right=Envelope<T#2>
/// @definition.implements symbol=Message source=Envelope<T> target=Envelope<T#2>
/// @definition.conformance symbol=Message member=Envelope.Label requirement=Envelope.Label
/// @type.symbol symbol=Message.T source="T: string" type=T#2
/// @resolution.name source=Envelope target=Envelope
/// @resolution.name source=T target=Message.T

type EventLabel = Message<"orders">.Label<"created">;
/// @type.symbol symbol=EventLabel source="type EventLabel = Message<\"orders\">.Label<\"created\">" type="orders:created"
/// @definition.type symbol=EventLabel source="type EventLabel = Message<\"orders\">.Label<\"created\">" value="orders:created"
/// @resolution.name source="Message<\"orders\">.Label<\"created\">" target=Envelope.Label
/// @resolution.name source=Message target=Message
/// @generic.instance id="Envelope<\"orders\">" template=Envelope arguments=("orders")
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
/// @type.symbol symbol=Message source="class Message<T: string> implements Envelope<T> {}" type=Message
/// @definition.class symbol=Message source="class Message<T: string> implements Envelope<T> {}" template=(in out T: string)
/// @definition.where symbol=Message source=Envelope<T> relation=satisfies left=this right=envelope.Envelope<T>
/// @definition.implements symbol=Message source=Envelope<T> target=envelope.Envelope<T>
/// @definition.conformance symbol=Message member=envelope.Envelope.Label requirement=envelope.Envelope.Label
/// @type.symbol symbol=Message.T source="T: string" type=T
/// @resolution.name source=Envelope target=envelope.Envelope
/// @resolution.name source=T target=Message.T

type EventLabel = Message<"orders">.Label<"created">;
/// @type.symbol symbol=EventLabel source="type EventLabel = Message<\"orders\">.Label<\"created\">" type="orders:created"
/// @definition.type symbol=EventLabel source="type EventLabel = Message<\"orders\">.Label<\"created\">" value="orders:created"
/// @resolution.name source="Message<\"orders\">.Label<\"created\">" target=envelope.Envelope.Label
/// @resolution.name source=Message target=Message
/// @generic.instance id="Message<\"orders\">" template=Message arguments=("orders")
/// @generic.instance id="envelope.Envelope<\"orders\">" template=envelope.Envelope arguments=("orders")
"#,
    );
}
