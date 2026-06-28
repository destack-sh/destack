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
=== annotated ===
declare const bytes: uint8[];
declare const index: usize;
const byte: uint8 = bytes[index];

=== checked ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=Array<uint8>

declare const index: usize;
/// @type.symbol symbol=index source=index type=usize

const byte = bytes[index];
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.name source=bytes target=bytes
/// @resolution.call source=bytes[index] parameters=(usize) arguments=(provided(index) as usize) return=uint8 kind=symbol target=collections.array.index#7 receiver=Array<uint8>
/// @resolution.name source=index target=index
"#,
    );
}

#[test]
fn test_dynamic_array_subscript_write_selects_index_set() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: usize;
bytes[index] = 255;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: usize;
bytes[index] = 255;

=== checked ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=Array<uint8>

declare const index: usize;
/// @type.symbol symbol=index source=index type=usize

bytes[index] = 255;
/// @resolution.name source=bytes target=bytes
/// @resolution.pattern.assign source=bytes[index] kind=place place=subscript(collections.array.indexSet#4) type=255
/// @resolution.name source=index target=index
"#,
    );
}

#[test]
fn test_dynamic_array_compound_subscript_resolves_read_and_write() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: usize;
bytes[index] += 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: usize;
bytes[index] += 1;

=== checked ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=Array<uint8>

declare const index: usize;
/// @type.symbol symbol=index source=index type=usize

bytes[index] += 1;
/// @resolution.name source=bytes target=bytes
/// @resolution.call source="bytes[index] += 1" parameters=() return=uint8 kind=builtin builtin=binary.add
/// @resolution.pattern.assign source=bytes[index] kind=place place="subscript(collections.array.index#7, collections.array.indexSet#4)" type=uint8
/// @resolution.name source=index target=index
"#,
    );
}
