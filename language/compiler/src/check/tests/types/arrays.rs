use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_tuple_element_access() {
    let session = TestSession::single(
        r#"
const tuple = ["id", 42] as const;
const name = tuple[0];
const count = tuple[1];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
const tuple = ["id", 42] as const;
/// @type.symbol symbol=tuple type=readonly ["id", 42]

const name = tuple[0];
/// @resolution.name source=tuple target=tuple
/// @resolution.member source="tuple[0]" receiver=readonly ["id", 42] kind=symbol target=0
/// @type.symbol symbol=name type="id"

const count = tuple[1];
/// @resolution.name source=tuple target=tuple
/// @resolution.member source="tuple[1]" receiver=readonly ["id", 42] kind=symbol target=1
/// @type.symbol symbol=count type=42
"#,
    );
}

#[test]
fn test_check_records_fixed_array_element_access() {
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

#[test]
fn test_check_records_dynamic_array_element_access() {
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
/// @type.symbol symbol=bytes type=uint8[]

declare const index: usize;
/// @type.symbol symbol=index type=usize

const byte = bytes[index];
/// @resolution.name source=bytes target=bytes
/// @resolution.name source=index target=index
/// @resolution.member source="bytes[index]" receiver=uint8[] kind=builtin builtin=subscript.index
/// @type.symbol symbol=byte type=uint8
"#,
    );
}

#[test]
fn test_check_records_range_subscript_slices() {
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
