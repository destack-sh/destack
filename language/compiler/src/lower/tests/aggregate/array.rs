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
type Array<int32> {
    storage: slice<uninit<int32>, unique, exclusive, local>;
    count: isize;
    allocated: usize;
}

@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

constant string.0: String = "arrayFromSlice"

function test.main.build(): Array<int32> {
    local l0: [int32; 3], readonly

entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    local.set l0, v3
    v4: ref<[int32; 3], borrowed, readonly, frame> = local.address l0
    v5: uint64 = 0
    v6: usize = 3
    v7: slice<int32, borrowed, readonly, frame> = slice.view v4, v5, v6
    v8: slice<int32, borrowed, 'l0, readonly, local> = cast.bit v7 -> slice<int32, borrowed, 'l0, readonly, local>
    v9: Array<int32> = call arrayFromSlice<int32>(v8): <'a>(slice<int32, borrowed, 'a, readonly, local>) => Array<int32>
    return v9
}

function arrayFromSlice<int32, 'a>(v0: slice<int32, borrowed, 'a, readonly, local>): Array<int32> {
entry(v0: slice<int32, borrowed, 'a, readonly, local>):
    v1: ref<String, managed, mutable, local> = global.address string.0
    v2: ref<String, managed, mutable, undefined, local> = cast.bit v1 -> ref<String, managed, mutable, undefined, local>
    panic v2

b1:
    return
}

/// @layout.struct name=Array<int32> size=32 align=8
/// @layout.field owner=Array<int32> index=0 name=storage offset=0 size=16 align=8
/// @layout.field owner=Array<int32> index=1 name=count offset=16 size=8 align=8
/// @layout.field owner=Array<int32> index=2 name=allocated offset=24 size=8 align=8
/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
"#,
    );
}
