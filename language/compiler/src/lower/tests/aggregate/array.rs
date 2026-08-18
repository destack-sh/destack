use crate::tests::TestSession;

#[test]
fn test_lower_array_literals_construct_arrays() {
    let session = TestSession::single(
        r#"
function build(): ^int32[] {
    return [1, 2, 3];
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type destack.memory.unique.Unique<slice<uninit<int32>, managed, mutable>> = slice<uninit<int32>, unique, exclusive>;

type destack.collections.array.Array<int32> {
    storage: destack.memory.unique.Unique<slice<uninit<int32>, managed, mutable>>;
    count: isize;
    allocated: usize;
}

type destack.memory.unique.Unique<slice<uint8, managed, mutable>> = slice<uint8, unique, exclusive>;

type destack.string.string.String {
    bytes: destack.memory.unique.Unique<slice<uint8, managed, mutable>>;
}

immortal constant string.3441301661858404811.bytes: [uint8; 14] = b"arrayFromSlice"

immortal constant string.3441301661858404811: destack.string.string.String = {{globalAddress string.3441301661858404811.bytes, 14uint64}}

function test.main.build(): destack.collections.array.Array<int32> {
    local l0: [int32; 3], readonly

entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    local.set l0, v3
    v4: ref<[int32; 3], borrowed, readonly> = local.address l0
    v5: uint64 = 0
    v6: usize = 3
    v7: slice<int32, borrowed, readonly> = slice.view v4, v5, v6
    v8: destack.collections.array.Array<int32> = call destack.collections.array.arrayFromSlice<int32>(v7): <'a>(slice<int32, borrowed, 'a, readonly>) => destack.collections.array.Array<int32>
    return v8
}

function destack.collections.array.arrayFromSlice<int32, 'a>(v0: slice<int32, borrowed, 'a, readonly>): destack.collections.array.Array<int32> {
entry(v0: slice<int32, borrowed, 'a, readonly>):
    v1: ref<destack.string.string.String, managed, mutable> = global.address string.3441301661858404811
    v2: ref<destack.string.string.String, managed, mutable, undefined> = cast.bit v1 -> ref<destack.string.string.String, managed, mutable, undefined>
    panic v2

b1:
    return
}

/// @layout.struct name=destack.collections.array.Array<int32> size=32 align=8
/// @layout.field owner=destack.collections.array.Array<int32> index=0 name=storage offset=0 size=16 align=8
/// @layout.field owner=destack.collections.array.Array<int32> index=1 name=count offset=16 size=8 align=8
/// @layout.field owner=destack.collections.array.Array<int32> index=2 name=allocated offset=24 size=8 align=8
/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=bytes offset=0 size=16 align=8
"#,
    );
}
