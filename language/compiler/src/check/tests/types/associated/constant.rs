use crate::tests::{DirRows, TestSession};

#[test]
fn test_associated_constant_uses_static_type_argument() {
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
=== annotated ===
class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    type Lane = [uint8; this.Width];
}

declare const lane: Segment<string>.Lane;

=== checked ===
class Segment<Row> {
/// @generic.template symbol=Segment parameters=[Row]
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
/// @generic.instance source="Segment<string>" id=Segment<string>
/// @type.symbol symbol=lane type=[uint8; 8]

/// @generic.instance id=Segment<string> symbol=Segment arguments=[string]
"#,
    );
}

#[test]
fn test_associated_constant_refinement_flows_through_constraint() {
    let session = TestSession::single(
        r#"
interface RegisterBlock {
    comptime const Width: usize;

    read(): [uint8; this.Width];
}

function readHeader<T: RegisterBlock<comptime Width = 16>>(block: T): [uint8; 16] {
    return block.read();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
interface RegisterBlock {
    comptime const Width: usize;

    read(): [uint8; this.Width];
}

function readHeader<T: RegisterBlock<comptime Width = 16>>(block: T): [uint8; 16] {
    return block.read();
}

=== checked ===
interface RegisterBlock {
/// @type.symbol symbol=RegisterBlock type=RegisterBlock
/// @definition.interface symbol=RegisterBlock

    comptime const Width: usize;
    /// @type.symbol symbol=RegisterBlock.Width type=usize

    read(): [uint8; this.Width];
    /// @resolution.member source=this.Width receiver=RegisterBlock kind=symbol target=RegisterBlock.Width
    /// @type.symbol symbol=RegisterBlock.read type=(this: RegisterBlock) => [uint8; RegisterBlock.Width]
}

function readHeader<T: RegisterBlock<comptime Width = 16>>(block: T): [uint8; 16] {
/// @generic.template symbol=readHeader parameters=[T: RegisterBlock<comptime Width = 16>]
/// @type.symbol symbol=readHeader type=<T: RegisterBlock<comptime Width = 16>>(T) => [uint8; 16]
/// @resolution.name source=RegisterBlock target=RegisterBlock

    return block.read();
    /// @resolution.name source=block target=block
    /// @resolution.member source=block.read receiver=T kind=symbol target=RegisterBlock.read
    /// @resolution.call source=block.read() parameters=() return=[uint8; 16] kind=symbol target=RegisterBlock.read receiver=T
}
"#,
    );
}
