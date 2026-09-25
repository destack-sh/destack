use crate::tests::TestProgram;

/// Emit slice subviews and direct length projection from the physical descriptor.
#[test]
fn test_emit_slice() {
    let program = TestProgram::mir(
        r#"
export function subview<'a>(v0: slice<int32, borrowed, 'a, readonly>, v1: uint64, v2: uint64): uint64 {
entry(v0: slice<int32, borrowed, 'a, readonly>, v1: uint64, v2: uint64):
    v3: slice<int32, borrowed, 'a, readonly> = address (*v0)[v1; v2]
    v4: uint64 = slice.length v3
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function subview {
    address.add r4, r0, r2, 4
    move r5, r3
    move r0, r5
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64, i64, i64) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
    v5 = iconst.i64 4
    v6 = imul v3, v5  ; v5 = 4
    v7 = iadd v1, v6
    return v4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64, i64, i64) -> i64 native
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
