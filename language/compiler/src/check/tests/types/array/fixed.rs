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
/// @resolution.call source=bytes[1] parameters=(usize) arguments=(provided(1) as usize) return=uint8 kind=symbol target=collections.array.index#1 receiver=FixedArray<uint8, 4> instance="FixedArray<uint8, 4>.<extension#1>.index#1"
/// @generic.instance source=bytes[1] id="FixedArray<uint8, 4>.<extension#1>.index#1"

/// @generic.instance id="FixedArray<uint8, 4>.<extension#1>.index#1" template=collections.array.index#1 arguments=(uint8, 4, uint8, 4)
"#,
    );
}

#[test]
fn test_fixed_array_requires_explicit_copy_into_managed_array() {
    let session = TestSession::single(
        r#"
declare const fixed: [int32; 3];
const grown: int32[] = fixed;
const copied: int32[] = [...fixed];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const fixed: [int32; 3];
const grown: int32[] = fixed;
const copied: int32[] = [...fixed];

=== checked ===
declare const fixed: [int32; 3];
/// @type.symbol symbol=fixed source=fixed type=FixedArray<int32, 3>

const grown: int32[] = fixed;
/// @type.symbol symbol=grown source=grown type=Array<int32>
/// @resolution.name source=fixed target=fixed

const copied: int32[] = [...fixed];
/// @type.symbol symbol=copied source=copied type=Array<int32>
/// @resolution.name source=fixed target=fixed
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'FixedArray<int32, 3>' is not assignable to type 'Array<int32>'"
/// @diagnostic.label line=3 column=24 span="fixed" line_source="const grown: int32[] = fixed;"
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
