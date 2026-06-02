use crate::tests::{DirRows, TestSession};

#[test]
fn test_static_if_true_member_is_available() {
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
/// @generic.template symbol=Segment parameters=[Row]
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
/// @resolution.member source=narrow.narrow receiver=Segment<int32> kind=symbol target=Segment.narrow application=Segment<int32>
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
/// @resolution.member source=wide.wide receiver=Segment<string> kind=symbol target=Segment.wide application=Segment<string>
/// @type.node source=wide type=Segment<string>
/// @type.node source=wide.wide type=WideMeta
/// @generic.application id=Segment<int32> symbol=Segment arguments=[int32]
/// @generic.application id=Segment<string> symbol=Segment arguments=[string]
/// @static.entry value=Row
/// @static.entry value=int32
/// @static.entry value=string
"#);
}

#[test]
fn test_static_if_false_member_is_unavailable() {
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
/// @generic.template symbol=Segment parameters=[Row]
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
/// @generic.application id=Segment<string> symbol=Segment arguments=[string]
/// @static.entry value=Row
/// @static.entry value=string
"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'narrow'"
/// @diagnostic.label line=18 column=1 source="segment.narrow;"
"#,
    );
}
