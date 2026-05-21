use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_evaluates_associated_types_from_owner_generics() {
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
struct Box<T> {
/// @generic.slot symbol=Box.T index=0 kind=type
/// @type.symbol symbol=Box type=Box<T>

    type Item = T;
    /// @type.symbol symbol=Box.Item type=T

    value: T;
    /// @type.symbol symbol=Box.value type=T
}

declare const value: Box<string>.Item;
/// @resolution.name source=Box target=Box
/// @resolution.member source=Box<string>.Item receiver=Box<string> kind=symbol target=Box.Item
/// @instance.application source="Box<string>" id=Box<string>
/// @type.symbol symbol=value type=string

/// @instance.entry id=Box<string> symbol=Box arguments=[string]
"#,
    );
}

#[test]
fn test_check_evaluates_associated_constants_in_static_types() {
    let session = TestSession::single(
        r#"
class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    type Lane = [uint8; this.Width];
}

declare const lane: Segment<string>.Lane;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
class Segment<Row> {
/// @generic.slot symbol=Segment.Row index=0 kind=type
/// @type.symbol symbol=Segment type=Segment<Row>

    comptime const Width: uint = Row extends string ? 8 : 4;
    /// @type.symbol symbol=Segment.Width type=uint
    /// @static.symbol symbol=Segment.Width value="Row extends string ? 8 : 4"

    type Lane = [uint8; this.Width];
    /// @resolution.member source=this.Width receiver=Segment<Row> kind=symbol target=Segment.Width
    /// @type.symbol symbol=Segment.Lane type=[uint8; Segment.Width]
}

declare const lane: Segment<string>.Lane;
/// @resolution.name source=Segment target=Segment
/// @resolution.member source=Segment<string>.Lane receiver=Segment<string> kind=symbol target=Segment.Lane
/// @instance.application source="Segment<string>" id=Segment<string>
/// @type.symbol symbol=lane type=[uint8; 8]

/// @instance.entry id=Segment<string> symbol=Segment arguments=[string]
"#);
}

#[test]
fn test_check_reports_associated_types_in_value_positions() {
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
