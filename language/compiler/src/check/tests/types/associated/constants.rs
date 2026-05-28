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
/// @generic.application source="Segment<string>" id=Segment<string>
/// @type.symbol symbol=lane type=[uint8; 8]

/// @generic.instance id=Segment<string> symbol=Segment arguments=[string]
"#);
}
