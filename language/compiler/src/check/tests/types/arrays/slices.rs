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
=== annotated ===
declare const bytes: [uint8; 4];
const slice: Slice<uint8> = bytes[1..3];

=== checked ===
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>

const slice = bytes[1..3];
/// @type.symbol symbol=slice source=slice type=collections.slice.Slice<uint8>
/// @type.node source=bytes type=FixedArray<uint8, 4>
/// @type.node source=bytes[1..3] type=collections.slice.Slice<uint8>
/// @resolution.name source=bytes target=bytes
/// @resolution.call source=bytes[1..3] parameters=(unknown) return=collections.slice.Slice<uint8> kind=symbol target=collections.array.index#6 receiver=FixedArray<uint8, 4>
/// @type.node source=1 type=1
/// @type.node source=1..3 type=Range<1 | 3>
/// @type.node source=3 type=3
"#,
    );
}
