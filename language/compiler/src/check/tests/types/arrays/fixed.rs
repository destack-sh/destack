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
/// @type.symbol symbol=bytes source=bytes type=[uint8; 4]

const byte = bytes[1];
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.name source=bytes target=bytes
/// @resolution.member source=bytes[1] receiver=[uint8; 4] kind=builtin builtin=subscript.index
"#,
    );
}

#[test]
fn test_fixed_array_member_access_selects_size() {
    let session = TestSession::single(
        r#"
declare const bytes: [uint8; 4];
const size = bytes.size;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes source=bytes type=[uint8; 4]

const size = bytes.size;
/// @type.symbol symbol=size source=size type=usize
/// @resolution.name source=bytes target=bytes
/// @resolution.member source=bytes.size receiver=[uint8; 4] kind=symbol target=collections.array.FixedArray.size
/// @type.node source=bytes type=[uint8; 4]
/// @type.node source=bytes.size type=usize
"#,
    );
}
