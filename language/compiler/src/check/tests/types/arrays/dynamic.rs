use crate::tests::{DirRows, TestSession};

#[test]
fn test_dynamic_array_subscript_selects_element_type() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: usize;
const byte = bytes[index];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
declare const bytes: uint8[];
/// @type.symbol symbol=bytes type=Array<uint8>

declare const index: usize;
/// @type.symbol symbol=index type=usize

const byte = bytes[index];
/// @type.symbol symbol=byte type=uint8
/// @resolution.name source=bytes target=bytes
/// @resolution.member source=bytes[index] receiver=Array<uint8> kind=builtin builtin=subscript.index
/// @resolution.name source=index target=index
"#,
    );
}
