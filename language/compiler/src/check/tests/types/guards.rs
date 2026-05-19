use super::super::snapshot::{assert_check_snapshot, assert_check_snapshot_with_diagnostics};

#[test]
fn test_check_records_static_member_guards() {
    assert_check_snapshot(
        r#"
struct NarrowMeta {}
struct WideMeta {}

class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;

    @if(this.Width == 4)
    narrow: NarrowMeta;

    @if(this.Width == 8)
    wide: WideMeta;

    value: Row;
}

declare const narrow: Segment<int32>;
const narrowMeta = narrow.narrow;

declare const wide: Segment<string>;
const wideMeta = wide.wide;
"#,
        r#"
struct NarrowMeta {}
/// @type.symbol key=NarrowMeta value=NarrowMeta

struct WideMeta {}
/// @type.symbol key=WideMeta value=WideMeta

class Segment<Row> {
/// @generic.parameters key=Segment parameters=[Segment.Row]
/// @generic.parameter key=Segment.Row space=type
/// @type.symbol key=Segment value=Segment<Row>

    comptime const Width: uint = Row extends string ? 8 : 4;
    /// @type.symbol key=Segment.Width value=uint
    /// @type.static key=Segment.Width value=Row extends string ? 8 : 4

    @if(this.Width == 4)
    narrow: NarrowMeta;
    /// @type.symbol key=Segment.narrow value=NarrowMeta

    @if(this.Width == 8)
    wide: WideMeta;
    /// @type.symbol key=Segment.wide value=WideMeta

    value: Row;
    /// @type.symbol key=Segment.value value=Row
}

declare const narrow: Segment<int32>;
/// @type.symbol key=narrow value=Segment<int32>
/// @instance.node source="Segment<int32>" instance=instance0

const narrowMeta = narrow.narrow;
/// @resolution.name source=narrow target=narrow
/// @resolution.member source=narrow.narrow receiver=Segment<int32> kind=direct target=Segment.narrow
/// @type.symbol key=narrowMeta value=NarrowMeta

declare const wide: Segment<string>;
/// @type.symbol key=wide value=Segment<string>
/// @instance.node source="Segment<string>" instance=instance1

const wideMeta = wide.wide;
/// @resolution.name source=wide target=wide
/// @resolution.member source=wide.wide receiver=Segment<string> kind=direct target=Segment.wide
/// @type.symbol key=wideMeta value=WideMeta

/// @instance.entry instance=instance0 key=Segment arguments=[int32]
/// @instance.entry instance=instance1 key=Segment arguments=[string]
/// @type.summary types=7 nodes=0 symbols=10
/// @generic.summary parameters=1 lists=1
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=2 labels=0 members=2 calls=0
/// @instance.summary instances=2 nodes=2
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

#[test]
fn test_check_reports_removed_static_members() {
    assert_check_snapshot_with_diagnostics(
        r#"
struct NarrowMeta {}
struct WideMeta {}

class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;

    @if(this.Width == 4)
    narrow: NarrowMeta;

    @if(this.Width == 8)
    wide: WideMeta;

    value: Row;
}

declare const segment: Segment<string>;
segment.narrow;
"#,
        r#"
=== dir ===
struct NarrowMeta {}
/// @type.symbol key=NarrowMeta value=NarrowMeta

struct WideMeta {}
/// @type.symbol key=WideMeta value=WideMeta

class Segment<Row> {
/// @generic.parameters key=Segment parameters=[Segment.Row]
/// @generic.parameter key=Segment.Row space=type
/// @type.symbol key=Segment value=Segment<Row>

    comptime const Width: uint = Row extends string ? 8 : 4;
    /// @type.symbol key=Segment.Width value=uint
    /// @type.static key=Segment.Width value=Row extends string ? 8 : 4

    @if(this.Width == 4)
    narrow: NarrowMeta;
    /// @type.symbol key=Segment.narrow value=NarrowMeta

    @if(this.Width == 8)
    wide: WideMeta;
    /// @type.symbol key=Segment.wide value=WideMeta

    value: Row;
    /// @type.symbol key=Segment.value value=Row
}

declare const segment: Segment<string>;
/// @type.symbol key=segment value=Segment<string>
/// @instance.node source="Segment<string>" instance=instance0

segment.narrow;
/// @resolution.name source=segment target=segment

/// @instance.entry instance=instance0 key=Segment arguments=[string]
/// @type.summary types=6 nodes=0 symbols=7
/// @generic.summary parameters=1 lists=1
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=0 calls=0
/// @instance.summary instances=1 nodes=1
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0

=== diagnostics ===
/// @diagnostic.error code=EC300 message="missing member 'narrow'"
/// @diagnostic.label line=18 column=9 source="segment.narrow;"
"#,
    );
}
