use crate::tests::TestProgram;

/// Emit exact counter and sample instrumentation sites.
#[test]
fn test_emit_profile() {
    let program = TestProgram::mir(
        r#"
export function observe(v0: int32): int32 {
entry(v0: int32):
    profile.increment counter(0)
    profile.sample sampler(0), v0
    return v0
}
"#,
    );

    program.assert_bytecode(
        r#"
function observe {
    profile.increment c0
    profile.sample s0, r0
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    gv3 = symbol colocated userextname1
    sig0 = (i64, i32) native
    sig1 = (i64, i32, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = symbol_value.i64 gv2
    v3 = load.i32 notrap aligned v2
    v4 = load.i64 notrap aligned region0 v0+8
    v5 = load.i64 notrap aligned v4+104
    call_indirect sig0, v5(v0, v3)
    v6 = symbol_value.i64 gv3
    v7 = load.i32 notrap aligned v6
    v8 = uextend.i64 v1
    v9 = load.i64 notrap aligned region0 v0+8
    v10 = load.i64 notrap aligned v9+112
    call_indirect sig1, v10(v0, v7, v8)
    return v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}
