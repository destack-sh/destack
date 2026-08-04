use crate::tests::TestProgram;

/// Advance one slice reference by its element stride and replace its length.
#[test]
fn test_emit_native_slice_view() {
    let program = TestProgram::mir(
        r#"
export function subview(
    v0: slice<int32, borrowed, readonly>,
    v1: uint64,
    v2: uint64,
): uint64 {
entry(v0: slice<int32, borrowed, readonly>, v1: uint64, v2: uint64):
    v3: slice<int32, borrowed, readonly> = slice.view v0, v1, v2
    v4: uint64 = slice.length v3
    return v4
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i64, i64, i64) -> i64 native {
block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
    v5 = iconst.i64 4
    v6 = imul v3, v5  ; v5 = 4
    v7 = iadd v1, v6
    return v4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64, i64, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i64 notrap aligned v1+16
    v6 = load.i64 notrap aligned v1+24
    v7 = call fn0(v0, v3, v4, v5, v6)
    store notrap aligned v7, v2
    return
}
"#,
    );
}
