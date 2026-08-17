use crate::tests::{DirRows, TestSession};

#[test]
fn test_dynamic_array_subscript_selects_element_type() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: isize;
const byte = bytes[index];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: isize;
const byte: uint8 = bytes[index];

=== dir ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=Array<uint8>
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @generic.instance id=Array<uint8> template=collections.array.Array arguments=(uint8)
/// @generic.instance id=memory.init.MaybeUninit<uint8> template=memory.init.MaybeUninit arguments=(uint8)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<uint8>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<uint8>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<uint8>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<uint8>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<uint8>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<uint8>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<uint8>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<uint8>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<uint8>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<uint8>)

declare const index: isize;
/// @type.symbol symbol=index source=index type=isize
/// @resolution.pattern source=index kind=binding target=index

const byte = bytes[index];
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.pattern source=byte kind=binding target=byte
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.subscript source=bytes[index] type=uint8 kind=call target="collections.array.index#1(parameters=(isize), arguments=(provided(index) as isize), return=memory.type.WithAccess<&'static uint8, \"exclusive\">)"
/// @generic.instantiation id="collections.array.index#1<uint8, \"exclusive\">" template=collections.array.index#1 arguments=(uint8, "exclusive")
/// @generic.instance id="collections.array.index#1<uint8, \"exclusive\">" template=collections.array.index#1 arguments=(uint8, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame Array<uint8>, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame Array<uint8>, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame uint8, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame uint8, "exclusive")
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="local" lifetime="static" access="readonly"
/// @resolution.access source=index root=index
"#,
    );
}

#[test]
fn test_dynamic_array_subscript_write_selects_index_set() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: isize;
bytes[index] = 255;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: isize;
bytes[index] = 255;

=== dir ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=Array<uint8>
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @generic.instance id=Array<uint8> template=collections.array.Array arguments=(uint8)
/// @generic.instance id=memory.init.MaybeUninit<uint8> template=memory.init.MaybeUninit arguments=(uint8)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<uint8>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<uint8>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<uint8>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<uint8>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<uint8>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<uint8>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<uint8>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<uint8>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<uint8>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<uint8>)

declare const index: isize;
/// @type.symbol symbol=index source=index type=isize
/// @resolution.pattern source=index kind=binding target=index

bytes[index] = 255;
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.pattern.assign source=bytes[index] kind=place
/// @resolution.assignment source=bytes[index] write="collections.array.indexSet(parameters=(isize, uint8), arguments=(provided(index) as isize, write as uint8), return=void)" type=uint8
/// @generic.instantiation id=collections.array.indexSet<uint8> template=collections.array.indexSet arguments=(uint8)
/// @generic.instance id=collections.array.indexSet<uint8> template=collections.array.indexSet arguments=(uint8)
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="local" lifetime="static" access="readonly"
/// @resolution.access source=index root=index
"#,
    );
}

#[test]
fn test_dynamic_array_compound_subscript_resolves_read_and_write() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: isize;
bytes[index] += 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: isize;
bytes[index] += 1;

=== dir ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=Array<uint8>
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @generic.instance id=Array<uint8> template=collections.array.Array arguments=(uint8)
/// @generic.instance id=memory.init.MaybeUninit<uint8> template=memory.init.MaybeUninit arguments=(uint8)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<uint8>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<uint8>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<uint8>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<uint8>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<uint8>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<uint8>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<uint8>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<uint8>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<uint8>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<uint8>)

declare const index: isize;
/// @type.symbol symbol=index source=index type=isize
/// @resolution.pattern source=index kind=binding target=index

bytes[index] += 1;
/// @resolution.name source=bytes target=bytes
/// @resolution.operator source="bytes[index] += 1" type=uint8 operator="+" kind=builtin operands=[bytes[index] as uint8 families=(integer), 1 as uint8 families=(integer)]
/// @resolution.place source=bytes placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.pattern.assign source=bytes[index] kind=place
/// @resolution.assignment source=bytes[index] read="collections.array.index#1(parameters=(isize), arguments=(provided(index) as isize), return=memory.type.WithAccess<&'static uint8, \"exclusive\">)" write="collections.array.indexSet(parameters=(isize, uint8), arguments=(provided(index) as isize, write as uint8), return=void)" type=uint8
/// @generic.instantiation id="collections.array.index#1<uint8, \"exclusive\">" template=collections.array.index#1 arguments=(uint8, "exclusive")
/// @generic.instantiation id=collections.array.indexSet<uint8> template=collections.array.indexSet arguments=(uint8)
/// @generic.instance id="collections.array.index#1<uint8, \"exclusive\">" template=collections.array.index#1 arguments=(uint8, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame Array<uint8>, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame Array<uint8>, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame uint8, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame uint8, "exclusive")
/// @generic.instance id=collections.array.indexSet<uint8> template=collections.array.indexSet arguments=(uint8)
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="local" lifetime="static" access="readonly"
/// @resolution.access source=index root=index
"#,
    );
}
