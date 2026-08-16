use crate::tests::{DirRows, TestSession};

#[test]
fn test_range_subscript_selects_slice_type() {
    let session = TestSession::single(
        r#"
declare const bytes: [uint8; 4];
const slice = bytes[1..3];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const bytes: [uint8; 4];
const slice: [uint8] = bytes[1..3];

=== dir ===
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>
/// @resolution.pattern source=bytes kind=binding target=bytes

const slice = bytes[1..3];
/// @type.symbol symbol=slice source=slice type=Slice<uint8>
/// @resolution.pattern source=slice kind=binding target=slice
/// @type.node source=bytes type=FixedArray<uint8, 4>
/// @type.node source=bytes[1..3] type=Slice<uint8>
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="static" access="readonly"
/// @resolution.access source=bytes root=bytes
/// @resolution.subscript source=bytes[1..3] type=Slice<uint8> kind=call target="collections.fixed-array.index#2(parameters=(Range<isize>), arguments=(provided(1..3) as Range<isize>), return=&'static readonly Slice<uint8>)"
/// @generic.instantiation id="collections.fixed-array.index#2<uint8, 4, Range<isize>, \"readonly\">" template=collections.fixed-array.index#2 arguments=(uint8, 4, Range<isize>, "readonly")
/// @generic.instance id="FixedArray<uint8, 4>" template=collections.fixed-array.FixedArray arguments=(uint8, 4)
/// @generic.instance id="collections.fixed-array.index#2<uint8, 4, Range<isize>, \"readonly\">" template=collections.fixed-array.index#2 arguments=(uint8, 4, Range<isize>, "readonly")
/// @type.node source=1 type=1
/// @type.node source=1..3 type=Range<isize>
/// @generic.instance id=Range<isize> template=range.range.Range arguments=(isize)
/// @type.node source=3 type=3
"#,
    );
}
