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
/// @resolution.pattern source=bytes kind=binding target=bytes

declare const index: usize;
/// @type.symbol symbol=index source=index type=usize
/// @resolution.pattern source=index kind=binding target=index

const byte = bytes[index];
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.pattern source=byte kind=binding target=byte
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.subscript source=bytes[index] type=uint8 kind=call target="collections.array.index#1(parameters=(usize), arguments=(provided(index) as usize), return=memory.type.WithAccess<&'static uint8, \"exclusive\">)"
/// @generic.instance source=bytes[index] id="Array<uint8>.<extension#5>.index#1<\"exclusive\">"
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=index root=index

/// @generic.instance id="Array<uint8>.<extension#5>.index#1<\"exclusive\">" template=collections.array.index#1 arguments=(uint8, "exclusive")
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
/// @resolution.pattern source=bytes kind=binding target=bytes

declare const index: usize;
/// @type.symbol symbol=index source=index type=usize
/// @resolution.pattern source=index kind=binding target=index

bytes[index] = 255;
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.pattern.assign source=bytes[index] kind=place
/// @resolution.assignment source=bytes[index] write="collections.array.indexSet(parameters=(usize, uint8), arguments=(provided(index) as usize, write as uint8), return=void)" type=uint8
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=index root=index
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
/// @resolution.pattern source=bytes kind=binding target=bytes

declare const index: usize;
/// @type.symbol symbol=index source=index type=usize
/// @resolution.pattern source=index kind=binding target=index

bytes[index] += 1;
/// @resolution.name source=bytes target=bytes
/// @resolution.operator source="bytes[index] += 1" type=uint8 operator="+" kind=builtin operands=[bytes[index] as uint8 families=(integer), 1 as uint8 families=(integer)]
/// @resolution.place source=bytes placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.pattern.assign source=bytes[index] kind=place
/// @resolution.assignment source=bytes[index] read="collections.array.index#1(parameters=(usize), arguments=(provided(index) as usize), return=memory.type.WithAccess<&'static uint8, \"exclusive\">)" write="collections.array.indexSet(parameters=(usize, uint8), arguments=(provided(index) as usize, write as uint8), return=void)" type=uint8
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=index root=index
"#,
    );
}
