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
        DirRows::checked().with_statics(),
        r#"
struct NarrowMeta {}
/// @type.symbol symbol=NarrowMeta type=NarrowMeta

struct WideMeta {}
/// @type.symbol symbol=WideMeta type=WideMeta

class Segment<Row> {
/// @generic.slot symbol=Segment.Row index=0 kind=type
/// @type.symbol symbol=Segment type=Segment<Row>

    comptime const Width: uint = Row extends string ? 8 : 4;
    /// @type.symbol symbol=Segment.Width type=uint
    /// @static.symbol symbol=Segment.Width value="Row extends string ? 8 : 4"

    @if(this.Width == 4)
    narrow: NarrowMeta;
    /// @type.symbol symbol=Segment.narrow type=NarrowMeta

    @if(this.Width == 8)
    wide: WideMeta;
    /// @type.symbol symbol=Segment.wide type=WideMeta

    value: Row;
    /// @type.symbol symbol=Segment.value type=Row
}

declare const narrow: Segment<int32>;
/// @type.symbol symbol=narrow type=Segment<int32>
/// @generic.application source="Segment<int32>" id=Segment<int32>

const narrowMeta = narrow.narrow;
/// @resolution.name source=narrow target=narrow
/// @resolution.member source=narrow.narrow receiver=Segment<int32> kind=symbol target=Segment.narrow
/// @type.symbol symbol=narrowMeta type=NarrowMeta

declare const wide: Segment<string>;
/// @type.symbol symbol=wide type=Segment<string>
/// @generic.application source="Segment<string>" id=Segment<string>

const wideMeta = wide.wide;
/// @resolution.name source=wide target=wide
/// @resolution.member source=wide.wide receiver=Segment<string> kind=symbol target=Segment.wide
/// @type.symbol symbol=wideMeta type=WideMeta

/// @generic.instance id=Segment<int32> symbol=Segment arguments=[int32]
/// @generic.instance id=Segment<string> symbol=Segment arguments=[string]
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
        DirRows::checked().with_statics(),
        r#"
struct NarrowMeta {}
/// @type.symbol symbol=NarrowMeta type=NarrowMeta

struct WideMeta {}
/// @type.symbol symbol=WideMeta type=WideMeta

class Segment<Row> {
/// @generic.slot symbol=Segment.Row index=0 kind=type
/// @type.symbol symbol=Segment type=Segment<Row>

    comptime const Width: uint = Row extends string ? 8 : 4;
    /// @type.symbol symbol=Segment.Width type=uint
    /// @static.symbol symbol=Segment.Width value="Row extends string ? 8 : 4"

    @if(this.Width == 4)
    narrow: NarrowMeta;
    /// @type.symbol symbol=Segment.narrow type=NarrowMeta

    @if(this.Width == 8)
    wide: WideMeta;
    /// @type.symbol symbol=Segment.wide type=WideMeta

    value: Row;
    /// @type.symbol symbol=Segment.value type=Row
}

declare const segment: Segment<string>;
/// @type.symbol symbol=segment type=Segment<string>
/// @generic.application source="Segment<string>" id=Segment<string>

segment.narrow;
/// @resolution.name source=segment target=segment

/// @generic.instance id=Segment<string> symbol=Segment arguments=[string]
"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'narrow'"
/// @diagnostic.label line=18 column=9 source="segment.narrow;"
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
        DirRows::checked(),
        r#"
@if(false)
const hidden: MissingType = missingValue;

const visible = 1;
/// @type.symbol symbol=visible type=1
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
        DirRows::checked(),
        r#"
@if(true)
const value: int32 = "text";
/// @type.node source="\"text\"" type=string
/// @type.symbol symbol=value type=int32
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
        DirRows::checked(),
        r#"
let enabled = true;
/// @type.symbol symbol=enabled type=boolean

@if(enabled)
const value = 1;
/// @resolution.name source=enabled target=enabled

"#,
        r#"
/// @diagnostic.error code=EC401 message="static condition requires boolean value"
/// @diagnostic.label line=4 column=5 source="@if(enabled)"
"#,
    );
}
