use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_static_member_guards() {
    let session = TestSession::single(
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
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_statics(),
        r#"
struct NarrowMeta {}
/// @type.symbol symbol=NarrowMeta type=NarrowMeta
/// @type.node source="struct NarrowMeta {}" type=NarrowMeta

struct WideMeta {}
/// @type.symbol symbol=WideMeta type=WideMeta
/// @type.node source="struct WideMeta {}" type=WideMeta

class Segment<Row> {
/// @generic.slot symbol=Segment.Row index=0 kind=type
/// @type.symbol symbol=Segment type=Segment<Row>
/// @type.node type=Segment<Row>

    comptime const Width: uint = Row extends string ? 8 : 4;
    /// @type.symbol symbol=Segment.Width type=uint
    /// @static.symbol symbol=Segment.Width value="Row extends string ? 8 : 4"

    @if(this.Width == 4)
    /// @resolution.receiver source=this kind=this owner=Segment type=Segment<Row>
    /// @type.node source=this type=Segment<Row>

    narrow: NarrowMeta;
    /// @type.symbol symbol=Segment.narrow type=NarrowMeta
    /// @resolution.name source=NarrowMeta target=NarrowMeta

    @if(this.Width == 8)
    /// @resolution.receiver source=this kind=this owner=Segment type=Segment<Row>
    /// @type.node source=this type=Segment<Row>

    wide: WideMeta;
    /// @type.symbol symbol=Segment.wide type=WideMeta
    /// @resolution.name source=WideMeta target=WideMeta

    value: Row;
    /// @type.symbol symbol=Segment.value type=Row

}

declare const narrow: Segment<int32>;
/// @type.node source="declare const narrow: Segment<int32>" type=void
/// @type.symbol symbol=narrow type=Segment<int32>
/// @generic.application source=Segment<int32> id=Segment<int32>
/// @resolution.name source=Segment target=Segment

const narrowMeta = narrow.narrow;
/// @type.node source="const narrowMeta = narrow.narrow" type=void
/// @type.symbol symbol=narrowMeta type=NarrowMeta
/// @generic.application source=narrow.narrow id=Segment<int32>
/// @resolution.name source=narrow target=narrow
/// @resolution.member source=narrow.narrow receiver=Segment<int32> kind=symbol target=Segment.narrow instance=Segment<int32>
/// @type.node source=narrow type=Segment<int32>
/// @type.node source=narrow.narrow type=NarrowMeta

declare const wide: Segment<string>;
/// @type.node source="declare const wide: Segment<string>" type=void
/// @type.symbol symbol=wide type=Segment<string>
/// @generic.application source=Segment<string> id=Segment<string>
/// @resolution.name source=Segment target=Segment

const wideMeta = wide.wide;
/// @type.node source="const wideMeta = wide.wide" type=void
/// @type.symbol symbol=wideMeta type=WideMeta
/// @generic.application source=wide.wide id=Segment<string>
/// @resolution.name source=wide target=wide
/// @resolution.member source=wide.wide receiver=Segment<string> kind=symbol target=Segment.wide instance=Segment<string>
/// @type.node source=wide type=Segment<string>
/// @type.node source=wide.wide type=WideMeta
/// @generic.instance id=Segment<int32> symbol=Segment arguments=[int32]
/// @generic.instance id=Segment<string> symbol=Segment arguments=[string]
/// @static.entry value=Row
/// @static.entry value=int32
/// @static.entry value=string
"#);
}

#[test]
fn test_check_reports_removed_static_members() {
    let session = TestSession::single(
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
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_statics(),
        r#"
struct NarrowMeta {}
/// @type.symbol symbol=NarrowMeta type=NarrowMeta
/// @type.node source="struct NarrowMeta {}" type=NarrowMeta

struct WideMeta {}
/// @type.symbol symbol=WideMeta type=WideMeta
/// @type.node source="struct WideMeta {}" type=WideMeta

class Segment<Row> {
/// @generic.slot symbol=Segment.Row index=0 kind=type
/// @type.symbol symbol=Segment type=Segment<Row>
/// @type.node type=Segment<Row>

    comptime const Width: uint = Row extends string ? 8 : 4;
    /// @type.symbol symbol=Segment.Width type=uint
    /// @static.symbol symbol=Segment.Width value="Row extends string ? 8 : 4"

    @if(this.Width == 4)
    /// @resolution.receiver source=this kind=this owner=Segment type=Segment<Row>
    /// @type.node source=this type=Segment<Row>

    narrow: NarrowMeta;
    /// @type.symbol symbol=Segment.narrow type=NarrowMeta
    /// @resolution.name source=NarrowMeta target=NarrowMeta

    @if(this.Width == 8)
    /// @resolution.receiver source=this kind=this owner=Segment type=Segment<Row>
    /// @type.node source=this type=Segment<Row>

    wide: WideMeta;
    /// @type.symbol symbol=Segment.wide type=WideMeta
    /// @resolution.name source=WideMeta target=WideMeta

    value: Row;
    /// @type.symbol symbol=Segment.value type=Row

}

declare const segment: Segment<string>;
/// @type.node source="declare const segment: Segment<string>" type=void
/// @type.symbol symbol=segment type=Segment<string>
/// @generic.application source=Segment<string> id=Segment<string>
/// @resolution.name source=Segment target=Segment

segment.narrow;
/// @resolution.name source=segment target=segment
/// @type.node source=segment type=Segment<string>
/// @generic.instance id=Segment<string> symbol=Segment arguments=[string]
/// @static.entry value=Row
/// @static.entry value=string
"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'narrow'"
/// @diagnostic.label line=18 column=1 source="segment.narrow;"
"#,
    );
}

#[test]
fn test_check_does_not_validate_false_static_if_branches() {
    let session = TestSession::single(
        r#"
@if(false)
const hidden: MissingType = missingValue;

const visible = 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
@if(false)
const hidden: MissingType = missingValue;

const visible = 1;
/// @type.node source="const visible = 1" type=void
/// @type.symbol symbol=visible type=int32
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_check_validates_true_static_if_branches() {
    let session = TestSession::single(
        r#"
@if(true)
const value: int32 = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
@if(true)
/// @type.node type=void

const value: int32 = "text";
/// @type.symbol symbol=value type=int32
/// @type.node source="\"text\"" type="text"
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=3 column=22 source="const value: int32 = \"text\";"
"#,
    );
}

#[test]
fn test_check_reports_non_boolean_static_if_conditions() {
    let session = TestSession::single(
        r#"
@if(1)
const value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
@if(1)
const value = 1;

"#,
        r#"
/// @diagnostic.error code=EC401 message="static condition requires boolean value"
/// @diagnostic.label line=2 column=5 source="@if(1)"
"#,
    );
}

#[test]
fn test_check_reports_runtime_static_if_conditions() {
    let session = TestSession::single(
        r#"
let enabled = true;

@if(enabled)
const value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let enabled = true;
/// @type.node source="let enabled = true" type=void
/// @type.symbol symbol=enabled type=boolean
/// @type.node source=true type=true

@if(enabled)
/// @resolution.name source=enabled target=enabled
/// @type.node source=enabled type=boolean

const value = 1;

"#,
        r#"
/// @diagnostic.error code=EC401 message="static condition requires boolean value"
/// @diagnostic.label line=4 column=5 source="@if(enabled)"
"#,
    );
}
