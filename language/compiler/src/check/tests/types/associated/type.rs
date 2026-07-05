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
struct Box<T> {
    type Item = T;
    value: T;
}

declare const value: Box<string>.Item;

=== checked ===
struct Box<T> {
/// @generic.template symbol=Box parameters=(T)
/// @type.symbol symbol=Box type=Box<T>

    type Item = T;
    /// @type.symbol symbol=Box.Item type=T

    value: T;
    /// @type.symbol symbol=Box.value type=T
}

declare const value: Box<string>.Item;
/// @resolution.name source=Box target=Box
/// @resolution.member source=Box<string>.Item receiver=Box<string> kind=symbol target=Box.Item
/// @generic.instance source="Box<string>" id=Box<string>
/// @type.symbol symbol=value type=string

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

const size = Packet.Size;

=== checked ===
class Packet {
/// @type.symbol symbol=Packet type=Packet

    type Size = uint32;
    /// @type.symbol symbol=Packet.Size type=uint32
}

const size = Packet.Size;
/// @resolution.name source=Packet target=Packet

"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'Size'"
/// @diagnostic.label line=6 column=21 source="const size = Packet.Size;"
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

    type Item;
    /// @type.symbol symbol=Iterator.Item type=Iterator.Item

    next(): this.Item;
    /// @resolution.member source=this.Item receiver=Iterator kind=symbol target=Iterator.Item
    /// @type.symbol symbol=Iterator.next type=(this: Iterator) => Iterator.Item
}

function nextByte<I: Iterator<type Item = uint8>>(iter: I): uint8 {
/// @generic.template symbol=nextByte parameters=(I: Iterator<type Item = uint8>)
/// @type.symbol symbol=nextByte type=<I: Iterator<type Item = uint8>>(I) => uint8
/// @resolution.name source=Iterator target=Iterator

    return iter.next();
    /// @resolution.name source=iter target=iter
    /// @resolution.member source=iter.next receiver=I kind=symbol target=Iterator.next
    /// @resolution.call source=iter.next() parameters=() return=uint8 kind=symbol target=Iterator.next receiver=I
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

    produce(): Factory.Output {
        return 7;
    }
}

type Made<F: Producing> = F.Output;

declare const made: Made<Factory>;

=== checked ===
interface Producing {
/// @generic.template symbol=Producing parameters=()
/// @type.symbol symbol=Producing type=Producing
/// @definition.interface symbol=Producing template=()
/// @definition.associated.type symbol=Producing.Output source="type Output" key=Output
/// @definition.method symbol=Producing.produce source="produce(): this.Output" slot=produce type=(this: Producing) => this.Output

    type Output;

    produce(): this.Output;
    /// @type.symbol symbol=Producing.produce source="produce(): this.Output" type=(this: Producing) => this.Output

}

class Factory implements Producing {
/// @generic.template symbol=Factory parameters=()
/// @type.symbol symbol=Factory type=Factory
/// @definition.class symbol=Factory template=()
/// @definition.where symbol=Factory source=Producing relation=satisfies left=this right=Producing
/// @definition.implements symbol=Factory source=Producing target=Producing
/// @definition.associated.type symbol=Factory.Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=Factory.produce slot=produce type=(this: Factory) => Factory.Output
/// @resolution.name source=Producing target=Producing

    type Output = int32;
    /// @type.symbol symbol=Factory.Output source="type Output = int32" type=int32

    produce(): this.Output {
    /// @type.symbol symbol=Factory.produce type=(this: Factory) => Factory.Output reduced=(this: Factory) => int32

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
/// @resolution.name source=Made target=Made
/// @resolution.name source=Factory target=Factory

/// @generic.instance id=Made<Factory> template=Made arguments=(Factory)
"#,
    );
}

#[test]
fn test_associated_type_default_flows_through_constraint() {
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

    session.assert_dir_checked(
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

    type Item = uint8;
    /// @type.symbol symbol=Iterator.Item type=uint8

    next(): this.Item;
    /// @resolution.member source=this.Item receiver=Iterator kind=symbol target=Iterator.Item
    /// @type.symbol symbol=Iterator.next type=(this: Iterator) => uint8
}

function nextDefault<I: Iterator>(iter: I): uint8 {
/// @generic.template symbol=nextDefault parameters=(I: Iterator)
/// @type.symbol symbol=nextDefault type=<I: Iterator>(I) => uint8
/// @resolution.name source=Iterator target=Iterator

    return iter.next();
    /// @resolution.name source=iter target=iter
    /// @resolution.member source=iter.next receiver=I kind=symbol target=Iterator.next
    /// @resolution.call source=iter.next() parameters=() return=uint8 kind=symbol target=Iterator.next receiver=I
}
"#,
    );
}
