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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
struct Box<out T> {
    type Item = T;
    value: T;
}

declare const value: Box<string>.Item;

=== checked ===
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
/// @type.symbol symbol=value source=value type=Box<string>.Item reduced=string
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Box target=Box

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Packet {
    type Size = uint32;
}

const size: uint32 = Packet.Size;

=== checked ===
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

    session.assert_dir_checked(
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

=== checked ===
interface Iterator {
/// @type.symbol symbol=Iterator type=Iterator
/// @definition.interface symbol=Iterator
/// @definition.associated.type symbol=Iterator.Item source="type Item" key=Item
/// @definition.method symbol=Iterator.next source="next(): this.Item" slot=next type=(this: this) => this.Item

    type Item;

    next(): this.Item;
    /// @type.symbol symbol=Iterator.next source="next(): this.Item" type=(this: this) => this.Item

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

    session.assert_dir_checked(
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

declare const made: Made<Factory>;

=== checked ===
interface Producing {
/// @type.symbol symbol=Producing type=Producing
/// @definition.interface symbol=Producing
/// @definition.associated.type symbol=Producing.Output source="type Output" key=Output
/// @definition.method symbol=Producing.produce source="produce(): this.Output" slot=produce type=(this: this) => this.Output

    type Output;

    produce(): this.Output;
    /// @type.symbol symbol=Producing.produce source="produce(): this.Output" type=(this: this) => this.Output

}

class Factory implements Producing {
/// @type.symbol symbol=Factory type=Factory
/// @definition.class symbol=Factory
/// @definition.where symbol=Factory source=Producing relation=satisfies left=this right=Producing
/// @definition.implements symbol=Factory source=Producing target=Producing
/// @definition.associated.type symbol=Factory.Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=Factory.produce slot=produce type=(this: this) => this.Output
/// @resolution.name source=Producing target=Producing

    type Output = int32;
    /// @type.symbol symbol=Factory.Output source="type Output = int32" type=int32

    produce(): this.Output {
    /// @type.symbol symbol=Factory.produce type=(this: this) => this.Output

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

declare const made: Made<Factory>;
/// @type.symbol symbol=made source=made type=Made<Factory> reduced=int32
/// @resolution.pattern source=made kind=binding target=made
/// @resolution.name source=Made target=Made
/// @resolution.name source=Factory target=Factory

/// @generic.instance id=Made<Factory> template=Made arguments=(Factory)
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
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
/// @diagnostic.error id=return-not-assignable message="type 'I.Item' is not assignable to the declared result type 'uint8'"
/// @diagnostic.label line=9 column=12 span="iter.next()" line_source="return iter.next();"
"#,
    );
}
