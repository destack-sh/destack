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
/// @resolution.pattern source=bytes kind=binding target=bytes

const byte = bytes[1];
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.pattern source=byte kind=binding target=byte
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.access source=bytes[1] root=bytes keys=[1]
/// @resolution.subscript source=bytes[1] type=uint8 kind=call target="collections.array.index#1(parameters=(usize), arguments=(provided(1) as usize), return=memory.type.WithAccess<&'static uint8, \"exclusive\">)"
/// @generic.instance source=bytes[1] id="FixedArray<uint8, 4>.<extension#1>.index#1<\"exclusive\">"

/// @generic.instance id="FixedArray<uint8, 4>.<extension#1>.index#1<\"exclusive\">" template=collections.array.index#1 arguments=(uint8, 4, "exclusive")
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
/// @resolution.pattern source=fixed kind=binding target=fixed

const grown: int32[] = fixed;
/// @type.symbol symbol=grown source=grown type=Array<int32>
/// @resolution.pattern source=grown kind=binding target=grown
/// @resolution.name source=fixed target=fixed
/// @resolution.place source=fixed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=fixed root=fixed

const copied: int32[] = [...fixed];
/// @type.symbol symbol=copied source=copied type=Array<int32>
/// @resolution.pattern source=copied kind=binding target=copied
/// @resolution.name source=fixed target=fixed
/// @resolution.place source=fixed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=fixed root=fixed
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'FixedArray<int32, 3>' is not assignable to type 'Array<int32>'"
/// @diagnostic.label line=3 column=24 span="fixed" line_source="const grown: int32[] = fixed;"
/// @diagnostic.related line=3 column=14 span="int32[]" line_source="const grown: int32[] = fixed;" message="expected due to this annotation"
/// @diagnostic.error id=not-assignable message="type 'FixedArray<int32, 3>' is not assignable to type 'int32'"
/// @diagnostic.label line=4 column=29 span="fixed" line_source="const copied: int32[] = [...fixed];"
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
/// @resolution.pattern source=bytes kind=binding target=bytes

const size = bytes.size;
/// @type.symbol symbol=size source=size type=usize
/// @resolution.pattern source=size kind=binding target=size
/// @type.node source=bytes type=FixedArray<uint8, 4>
/// @type.node source=bytes.size type=usize
/// @resolution.name source=bytes target=bytes
/// @resolution.member source=bytes.size receiver=FixedArray<uint8, 4> type=usize kind=call target="collections.array.size#1(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=bytes placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @generic.instance source=bytes.size id="FixedArray<uint8, 4>.<extension#1>.size#1"

/// @generic.instance id="FixedArray<uint8, 4>.<extension#1>.size#1" template=collections.array.size#1 arguments=(uint8, 4)
"#,
    );
}
