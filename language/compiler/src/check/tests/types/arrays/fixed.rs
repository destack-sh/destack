use crate::tests::{DirRows, TestSession};

#[test]
fn test_fixed_array_subscript_selects_element_type() {
    let session = TestSession::single(
        r#"
declare const bytes: [uint8; 4];
const byte = bytes[1];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes type=[uint8; 4]

const byte = bytes[1];
/// @resolution.name source=bytes target=bytes
/// @resolution.member source="bytes[1]" receiver=[uint8; 4] kind=builtin builtin=subscript.index
/// @type.symbol symbol=byte type=uint8
"#,
    );
}
