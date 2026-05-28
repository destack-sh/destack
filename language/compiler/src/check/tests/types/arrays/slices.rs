use crate::tests::{DirRows, TestSession};

#[test]
fn test_range_subscript_selects_slice_type() {
    let session = TestSession::single(
        r#"
declare const bytes: [uint8; 4];
const slice = bytes[1..3];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes type=[uint8; 4]

const slice = bytes[1..3];
/// @resolution.name source=bytes target=bytes
/// @type.node source=1..3 type=1..3
/// @resolution.member source="bytes[1..3]" receiver=[uint8; 4] kind=builtin builtin=subscript.slice
/// @type.symbol symbol=slice type=Slice<uint8>
"#,
    );
}
