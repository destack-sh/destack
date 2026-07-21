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
const slice: WithAccess<Borrowed<[uint8], "static", "mutable">, "readonly"> = bytes[1..3];

=== checked ===
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>
/// @resolution.pattern source=bytes kind=binding target=bytes

const slice = bytes[1..3];
/// @type.symbol symbol=slice source=slice type=memory.type.WithAccess<Borrowed<Slice<uint8>, "static", "mutable">, "readonly"> reduced=Borrowed<Slice<uint8>, "static", "readonly">
/// @resolution.pattern source=slice kind=binding target=slice
/// @type.node source=bytes type=FixedArray<uint8, 4>
/// @type.node source=bytes[1..3] type=memory.type.WithAccess<Borrowed<Slice<uint8>, "static", "mutable">, "readonly"> reduced=Borrowed<Slice<uint8>, "static", "readonly">
/// @resolution.name source=bytes target=bytes
/// @resolution.call source=bytes[1..3] parameters=(Range<usize>) arguments=(provided(1..3) as Range<usize>) return=memory.type.WithAccess<Borrowed<Slice<uint8>, "static", "mutable">, "readonly"> kind=symbol target=collections.array.index#3 receiver=FixedArray<uint8, 4> adjustments=(borrow) instance="FixedArray<uint8, 4>.<extension#3>.index#3"
/// @generic.instance source=bytes[1..3] id="FixedArray<uint8, 4>.<extension#3>.index#3"
/// @generic.instance source=bytes[1..3] id="memory.type.WithAccess<Borrowed<Slice<uint8>, \"static\", \"mutable\">, \"readonly\">"
/// @type.node source=1 type=1
/// @type.node source=1..3 type=Range<usize>
/// @generic.instance source=1..3 id=Range<usize>
/// @type.node source=3 type=3

/// @generic.instance id="FixedArray<uint8, 4>.<extension#3>.index#3" template=collections.array.index#3 arguments=(uint8, 4, Range<usize>, uint8, 4, Range<usize>, "readonly")
/// @generic.instance id="memory.type.WithAccess<Borrowed<Slice<uint8>, \"static\", \"mutable\">, \"readonly\">" template=memory.type.WithAccess arguments=(Borrowed<Slice<uint8>, "static", "mutable">, "readonly")
/// @generic.instance id=Range<usize> template=range.range.Range arguments=(usize)
"#,
    );
}
