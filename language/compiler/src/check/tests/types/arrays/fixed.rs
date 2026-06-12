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
=== annotated ===
declare const bytes: [uint8; 4];
const byte: uint8 = bytes[1];

=== checked ===
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>

const byte = bytes[1];
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.name source=bytes target=bytes
/// @resolution.call source=bytes[1] parameters=(usize) return=uint8 kind=symbol target=collections.array.index#1 receiver=FixedArray<uint8, 4>
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
=== annotated ===
declare const bytes: [uint8; 4];
const size: usize = bytes.size;

=== checked ===
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>

const size = bytes.size;
/// @type.symbol symbol=size source=size type=usize
/// @type.node source=bytes type=FixedArray<uint8, 4>
/// @type.node source=bytes.size type=usize
/// @resolution.name source=bytes target=bytes
/// @resolution.member source=bytes.size receiver=FixedArray<uint8, 4> kind=symbol target=collections.array.size#1
"#,
    );
}
