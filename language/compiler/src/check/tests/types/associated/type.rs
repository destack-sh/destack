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
/// @generic.template symbol=Box parameters=[T]
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

/// @generic.instance id=Box<string> symbol=Box arguments=[string]
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

    next(): Option<this.Item>;
}

function nextByte<I: Iterator<type Item = uint8>>(iter: I): Option<uint8> {
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

    next(): Option<this.Item>;
}

function nextByte<I: Iterator<type Item = uint8>>(iter: I): Option<uint8> {
    return iter.next();
}

=== checked ===
interface Iterator {
/// @type.symbol symbol=Iterator type=Iterator
/// @definition.interface symbol=Iterator

    type Item;
    /// @type.symbol symbol=Iterator.Item type=Iterator.Item

    next(): Option<this.Item>;
    /// @resolution.name source=Option target=option.Option
    /// @resolution.member source=this.Item receiver=Iterator kind=symbol target=Iterator.Item
    /// @type.symbol symbol=Iterator.next type=(this: Iterator) => Option<Iterator.Item>
}

function nextByte<I: Iterator<type Item = uint8>>(iter: I): Option<uint8> {
/// @generic.template symbol=nextByte parameters=[I: Iterator<type Item = uint8>]
/// @type.symbol symbol=nextByte type=<I: Iterator<type Item = uint8>>(I) => Option<uint8>
/// @resolution.name source=Iterator target=Iterator
/// @resolution.name source=Option target=option.Option

    return iter.next();
    /// @resolution.name source=iter target=iter
    /// @resolution.member source=iter.next receiver=I kind=symbol target=Iterator.next
    /// @resolution.call source=iter.next() parameters=() return=Option<uint8> kind=symbol target=Iterator.next receiver=I
}
"#,
    );
}

#[test]
fn test_associated_type_default_flows_through_constraint() {
    let session = TestSession::single(
        r#"
interface Iterator {
    type Item = uint8;

    next(): Option<this.Item>;
}

function nextDefault<I: Iterator>(iter: I): Option<uint8> {
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

    next(): Option<this.Item>;
}

function nextDefault<I: Iterator>(iter: I): Option<uint8> {
    return iter.next();
}

=== checked ===
interface Iterator {
/// @type.symbol symbol=Iterator type=Iterator
/// @definition.interface symbol=Iterator

    type Item = uint8;
    /// @type.symbol symbol=Iterator.Item type=uint8

    next(): Option<this.Item>;
    /// @resolution.name source=Option target=option.Option
    /// @resolution.member source=this.Item receiver=Iterator kind=symbol target=Iterator.Item
    /// @type.symbol symbol=Iterator.next type=(this: Iterator) => Option<uint8>
}

function nextDefault<I: Iterator>(iter: I): Option<uint8> {
/// @generic.template symbol=nextDefault parameters=[I: Iterator]
/// @type.symbol symbol=nextDefault type=<I: Iterator>(I) => Option<uint8>
/// @resolution.name source=Iterator target=Iterator
/// @resolution.name source=Option target=option.Option

    return iter.next();
    /// @resolution.name source=iter target=iter
    /// @resolution.member source=iter.next receiver=I kind=symbol target=Iterator.next
    /// @resolution.call source=iter.next() parameters=() return=Option<uint8> kind=symbol target=Iterator.next receiver=I
}
"#,
    );
}
