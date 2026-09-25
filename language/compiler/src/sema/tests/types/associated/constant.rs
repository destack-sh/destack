use crate::tests::{DirRows, TestSession};

/// Infer associated constant types from literal initializers.
#[test]
fn test_infer_literal_associated_constants() {
    let session = TestSession::single(
        r#"
class Class {
    const Value = 1;
}

interface Interface {
    const Value = 1;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
class Class {
    const Value = 1;
}

interface Interface {
    const Value = 1;
}

=== dir ===
class Class {
/// @type.symbol symbol=Class type=typeof Class
/// @definition.class symbol=Class
/// @definition.associated.const symbol=Class.Value source="const Value = 1" key=Value type=1

    const Value = 1;
    /// @type.symbol symbol=Class.Value source="const Value = 1" type=1
    /// @static.symbol symbol=Class.Value source="const Value = 1" value=1

}

interface Interface {
/// @generic.template symbol=Interface parameters=(this: Interface)
/// @type.symbol symbol=Interface type=Interface
/// @definition.interface symbol=Interface template=(this: Interface)
/// @definition.where symbol=Interface relation=satisfies left=this right=Interface
/// @definition.associated.const symbol=Interface.Value source="const Value = 1" key=Value type=1

    const Value = 1;
    /// @type.symbol symbol=Interface.Value source="const Value = 1" type=1
    /// @static.symbol symbol=Interface.Value source="const Value = 1" value=1

}
"#,
    );
}

/// Report associated constant types that need inference.
#[test]
fn test_report_associated_constants_needing_inference() {
    let session = TestSession::single(
        r#"
class Class {
    const Value;
    const Computed = 1 + 2;
}

interface Interface {
    const Value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
class Class {
    const Value;
    const Computed = 1 + 2;
}

interface Interface {
    const Value;
}

=== dir ===
class Class {
/// @type.symbol symbol=Class type=typeof Class
/// @definition.class symbol=Class
/// @definition.associated.const symbol=Class.Computed source="const Computed = 1 + 2" key=Computed type=3
/// @definition.associated.const symbol=Class.Value source="const Value" key=Value type=<error>

    const Value;
    /// @type.symbol symbol=Class.Value source="const Value" type=<error>

    const Computed = 1 + 2;
    /// @type.symbol symbol=Class.Computed source="const Computed = 1 + 2" type=3
    /// @static.symbol symbol=Class.Computed source="const Computed = 1 + 2" value=3

}

interface Interface {
/// @generic.template symbol=Interface parameters=(this: Interface)
/// @type.symbol symbol=Interface type=Interface
/// @definition.interface symbol=Interface template=(this: Interface)
/// @definition.where symbol=Interface relation=satisfies left=this right=Interface
/// @definition.associated.const symbol=Interface.Value source="const Value" key=Value type=<error>

    const Value;
    /// @type.symbol symbol=Interface.Value source="const Value" type=<error>

}
"#,
        r#"
/// @diagnostic.error id=missing-type-annotation message="missing type annotation"
/// @diagnostic.label line=3 column=11 span="Value" line_source="const Value;"
/// @diagnostic.error id=missing-type-annotation message="missing type annotation"
/// @diagnostic.label line=8 column=11 span="Value" line_source="const Value;"
"#,
    );
}

#[test]
fn test_associated_constant_uses_static_type_argument() {
    let session = TestSession::single(
        r#"
class Segment<in out Row> {
    const Width: uint = Row extends string ? 8 : 4;
    type Lane = [uint8; this.Width];
}

declare const lane: Segment<string>.Lane;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
class Segment<in out Row> {
    const Width: uint = Row extends string ? 8 : 4;
    type Lane = [uint8; this.Width];
}

declare const lane: [uint8; 8];

=== dir ===
class Segment<in out Row> {
/// @generic.template symbol=Segment parameters=(in out Row)
/// @type.symbol symbol=Segment type=typeof Segment
/// @definition.class symbol=Segment template=(in out Row)
/// @definition.associated.type symbol=Segment.Lane source="type Lane = [uint8; this.Width]" key=Lane value="FixedArray<uint8, this.Width>"
/// @definition.associated.const symbol=Segment.Width source="const Width: uint = Row extends string ? 8 : 4" key=Width type=uint64
/// @type.symbol symbol=Segment.Row source="in out Row" type=Row

    const Width: uint = Row extends string ? 8 : 4;
    /// @type.symbol symbol=Segment.Width source="const Width: uint = Row extends string ? 8 : 4" type=uint64
    /// @static.symbol symbol=Segment.Width source="const Width: uint = Row extends string ? 8 : 4" value="Row extends string ? 8 : 4"
    /// @resolution.name source=Row target=Segment.Row

    type Lane = [uint8; this.Width];
    /// @type.symbol symbol=Segment.Lane source="type Lane = [uint8; this.Width]" type=FixedArray<uint8, this.Width>

}

declare const lane: Segment<string>.Lane;
/// @type.symbol symbol=lane source=lane type=FixedArray<uint8, 8>
/// @resolution.pattern source=lane kind=binding target=lane
/// @resolution.name source=Segment target=Segment
/// @resolution.name source=Segment<string>.Lane target=Segment.Lane
/// @generic.instance id=Segment<string> template=Segment arguments=(string)
"#,
    );
}

#[test]
fn test_associated_constant_refinement_flows_through_constraint() {
    let session = TestSession::single(
        r#"
interface RegisterBlock {
    const Width: usize;

    read(): [uint8; this.Width];
}

function readHeader<T: RegisterBlock<const Width = 16>>(block: T): [uint8; 16] {
    return block.read();
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
interface RegisterBlock {
    const Width: usize;

    read(): [uint8; this.Width];
}

function readHeader<T: RegisterBlock<const Width = 16>>(block: T): [uint8; 16] {
    return block.read();
}

=== dir ===
interface RegisterBlock {
/// @generic.template symbol=RegisterBlock parameters=(this: RegisterBlock)
/// @type.symbol symbol=RegisterBlock type=RegisterBlock
/// @definition.interface symbol=RegisterBlock template=(this: RegisterBlock)
/// @definition.where symbol=RegisterBlock relation=satisfies left=this right=RegisterBlock
/// @definition.associated.const symbol=RegisterBlock.Width source="const Width: usize" key=Width type=usize
/// @definition.method symbol=RegisterBlock.read source="read(): [uint8; this.Width]" slot=read type=() => FixedArray<uint8, this.Width>

    const Width: usize;
    /// @type.symbol symbol=RegisterBlock.Width source="const Width: usize" type=usize

    read(): [uint8; this.Width];
    /// @type.symbol symbol=RegisterBlock.read source="read(): [uint8; this.Width]" type=() => FixedArray<uint8, this.Width>

}

function readHeader<T: RegisterBlock<const Width = 16>>(block: T): [uint8; 16] {
/// @generic.template symbol=readHeader parameters=(T: RegisterBlock<type Width = 16>)
/// @type.symbol symbol=readHeader type=<T: RegisterBlock<type Width = 16>>(T) => FixedArray<uint8, 16>
/// @type.symbol symbol=readHeader.T source="T: RegisterBlock<const Width = 16>" type=T
/// @resolution.name source=RegisterBlock target=RegisterBlock
/// @resolution.name source="const Width = 16" target=RegisterBlock.Width
/// @type.symbol symbol=readHeader.block source="block: T" type=T
/// @resolution.name source=T target=readHeader.T

    return block.read();
    /// @resolution.name source=block target=readHeader.block
    /// @resolution.member source=block.read receiver=T type=() => FixedArray<uint8, 16> kind=symbol target_receiver=T target=RegisterBlock.read
    /// @resolution.call source=block.read() parameters=() return=FixedArray<uint8, 16> kind=symbol target=RegisterBlock.read receiver=T
    /// @resolution.place source=block placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=block root=readHeader.block

}
"#,
    );
}

/// Use an interface associated constant default for an extension conformance.
#[test]
fn test_use_associated_constant_default_for_extension_conformance() {
    let session = TestSession::single(
        r#"
interface Shape {
    const Rank: usize = 2;
}

struct Matrix {}

extension of Matrix implements Shape {}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked().with_statics(), r#"
=== annotated ===
interface Shape {
    const Rank: usize = 2;
}

struct Matrix {}

extension of Matrix implements Shape {}

=== dir ===
interface Shape {
/// @generic.template symbol=Shape parameters=(this: Shape)
/// @type.symbol symbol=Shape type=Shape
/// @definition.interface symbol=Shape template=(this: Shape)
/// @definition.where symbol=Shape relation=satisfies left=this right=Shape
/// @definition.associated.const symbol=Shape.Rank source="const Rank: usize = 2" key=Rank type=usize

    const Rank: usize = 2;
    /// @type.symbol symbol=Shape.Rank source="const Rank: usize = 2" type=usize
    /// @static.symbol symbol=Shape.Rank source="const Rank: usize = 2" value=2

}

struct Matrix {}
/// @type.symbol symbol=Matrix source="struct Matrix {}" type=Matrix
/// @definition.struct symbol=Matrix source="struct Matrix {}"

extension of Matrix implements Shape {}
/// @definition.extension symbol=<module>#2 source="extension of Matrix implements Shape {}" form=local target=Matrix
/// @definition.implements symbol=<module>#2 source=Shape target=Shape
/// @definition.conformance symbol=<module>#2 member=Shape.Rank requirement=Shape.Rank
/// @resolution.name source=Matrix target=Matrix
/// @resolution.name source=Shape target=Shape
"#);
}
