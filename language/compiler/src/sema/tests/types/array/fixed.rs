use crate::tests::{DirRows, TestSession};

#[test]
fn test_fixed_array_subscript_selects_element_type() {
    let session = TestSession::single(
        r#"
declare const bytes: [uint8; 4];
const byte = bytes[1];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: [uint8; 4];
const byte: uint8 = bytes[1];

=== dir ===
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>
/// @resolution.pattern source=bytes kind=binding target=bytes

const byte = bytes[1];
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.pattern source=byte kind=binding target=byte
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=bytes root=bytes
/// @resolution.access source=bytes[1] root=bytes keys=[1]
/// @resolution.subscript source=bytes[1] type=uint8 kind=call target="index#1(parameters=(isize), arguments=(provided(1) as isize), return=WithAccess<&'static constant uint8, \"readonly\">, regions=(\"static\" & \"constant\"))"
/// @generic.instantiation id="index#1<uint8, 4, \"readonly\">" template=index#1 arguments=(uint8, 4, "readonly")
/// @generic.instance id="WithAccess<&'bound0 FixedArray<uint8, 4>, \"readonly\">" template=WithAccess arguments=(&'bound0 FixedArray<uint8, 4>, "readonly")
/// @generic.instance id="WithAccess<&'bound0 uint8, \"readonly\">" template=WithAccess arguments=(&'bound0 uint8, "readonly")
/// @generic.instance id="index#1<uint8, 4, \"readonly\">" template=index#1 arguments=(uint8, 4, "readonly")
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const fixed: [int32; 3];
const grown: int32[] = fixed;
const copied: int32[] = [...fixed];

=== dir ===
declare const fixed: [int32; 3];
/// @type.symbol symbol=fixed source=fixed type=FixedArray<int32, 3>
/// @resolution.pattern source=fixed kind=binding target=fixed

const grown: int32[] = fixed;
/// @type.symbol symbol=grown source=grown type=int32[]
/// @resolution.pattern source=grown kind=binding target=grown
/// @resolution.name source=fixed target=fixed
/// @resolution.place source=fixed placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=fixed root=fixed

const copied: int32[] = [...fixed];
/// @type.symbol symbol=copied source=copied type=int32[]
/// @resolution.pattern source=copied kind=binding target=copied
/// @resolution.call source=[...fixed] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @resolution.name source=fixed target=fixed
/// @resolution.place source=fixed placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=fixed root=fixed
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'FixedArray<int32, 3>' is not assignable to type 'int32[]'"
/// @diagnostic.label line=3 column=24 span="fixed" line_source="const grown: int32[] = fixed;"
/// @diagnostic.related line=3 column=14 span="int32[]" line_source="const grown: int32[] = fixed;" message="expected due to this annotation"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const bytes: [uint8; 4];
const size: isize = bytes.size;

=== dir ===
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>
/// @resolution.pattern source=bytes kind=binding target=bytes

const size = bytes.size;
/// @type.symbol symbol=size source=size type=isize
/// @resolution.pattern source=size kind=binding target=size
/// @type.node source=bytes type=FixedArray<uint8, 4>
/// @type.node source=bytes.size type=isize
/// @resolution.name source=bytes target=bytes
/// @resolution.member source=bytes.size receiver=FixedArray<uint8, 4> type=isize kind=call target="size(parameters=(), arguments=(), return=isize, regions=(\"static\" & \"constant\"))"
/// @resolution.place source=bytes placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=bytes root=bytes
/// @generic.instantiation id="size<uint8, 4>" template=size arguments=(uint8, 4)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="FixedArray<uint8, 4>" template=FixedArray arguments=(uint8, 4)
/// @generic.instance id="size<uint8, 4>" template=size arguments=(uint8, 4)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
"#,
    );
}
