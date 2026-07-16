use crate::tests::{DirRows, TestSession};

#[test]
fn test_associated_constant_uses_static_type_argument() {
    let session = TestSession::single(
        r#"
class Segment<in out Row> {
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
class Segment<in out Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    type Lane = [uint8; this.Width];
}

declare const lane: Segment<string>.Lane;

=== checked ===
class Segment<in out Row> {
/// @generic.template symbol=Segment parameters=(in out Row)
/// @type.symbol symbol=Segment type=Segment
/// @definition.class symbol=Segment template=(in out Row)
/// @definition.associated.type symbol=Segment.Lane source="type Lane = [uint8; this.Width]" key=Lane value="FixedArray<uint8, this.Width>"
/// @definition.associated.const symbol=Segment.Width source="comptime const Width: uint = Row extends string ? 8 : 4" key=Width type=uint64
/// @type.symbol symbol=Segment.Row source="in out Row" type=Row

    comptime const Width: uint = Row extends string ? 8 : 4;
    /// @type.symbol symbol=Segment.Width source="comptime const Width: uint = Row extends string ? 8 : 4" type=uint64
    /// @resolution.name source=Row target=Segment.Row

    type Lane = [uint8; this.Width];
    /// @type.symbol symbol=Segment.Lane source="type Lane = [uint8; this.Width]" type=FixedArray<uint8, this.Width>

}

declare const lane: Segment<string>.Lane;
/// @type.symbol symbol=lane source=lane type=Segment<string>.Lane reduced=FixedArray<uint8, 8>
/// @resolution.name source=Segment target=Segment

/// @generic.instance id=Segment<string> template=Segment arguments=(string)
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
/// @definition.associated.const symbol=RegisterBlock.Width source="comptime const Width: usize" key=Width type=usize
/// @definition.method symbol=RegisterBlock.read source="read(): [uint8; this.Width]" slot=read type=(this: this) => FixedArray<uint8, this.Width>

    comptime const Width: usize;
    /// @type.symbol symbol=RegisterBlock.Width source="comptime const Width: usize" type=usize

    read(): [uint8; this.Width];
    /// @type.symbol symbol=RegisterBlock.read source="read(): [uint8; this.Width]" type=(this: this) => FixedArray<uint8, this.Width>

}

function readHeader<T: RegisterBlock<comptime Width = 16>>(block: T): [uint8; 16] {
/// @generic.template symbol=readHeader parameters=(T: RegisterBlock<type Width = 16>)
/// @type.symbol symbol=readHeader type=<T: RegisterBlock<type Width = 16>>(T) => FixedArray<uint8, 16>
/// @type.symbol symbol=readHeader.T source="T: RegisterBlock<comptime Width = 16>" type=T
/// @resolution.name source=RegisterBlock target=RegisterBlock
/// @type.symbol symbol=readHeader.block source="block: T" type=T
/// @resolution.name source=T target=readHeader.T

    return block.read();
    /// @resolution.name source=block target=readHeader.block
    /// @resolution.member source=block.read receiver=T kind=symbol target=RegisterBlock.read
    /// @resolution.call source=block.read() parameters=() return=FixedArray<uint8, T.Width> kind=symbol target=RegisterBlock.read receiver=T

}
"#,
    );
}
