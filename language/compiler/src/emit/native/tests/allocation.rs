use crate::tests::TestProgram;

/// Allocate fixed and repeated managed heap storage through the runtime ABI.
#[test]
fn test_emit_native_allocation() {
    let program = TestProgram::mir(
        r#"
export function allocate(v0: int64): slice<int32, unique, mutable> {
entry(v0: int64):
    v1: ref<int32, unique, mutable> = new.zeroed int32
    v2: slice<int32, unique, mutable> = new.slice.zeroed int32, v0
    free v1
    return v2
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64) -> i64, i64 native {
    gv0 = symbol colocated userextname0
    gv1 = symbol colocated userextname1
    sig0 = (i64, i32, i32, i32) -> i64 native
    sig1 = (i64, i32, i32, i64, i32) -> i64 native
    sig2 = (i64, i32, i64) native

block0(v0: i64, v1: i64):
    v2 = iconst.i32 0
    v3 = symbol_value.i64 gv0
    v4 = load.i32 notrap aligned v3
    v5 = iconst.i32 0
    v6 = load.i64 notrap aligned v0+8
    v7 = load.i64 notrap aligned v6
    v8 = call_indirect sig0, v7(v0, v2, v4, v5)  ; v2 = 0, v5 = 0
    v9 = iconst.i32 0
    v10 = symbol_value.i64 gv1
    v11 = load.i32 notrap aligned v10
    v12 = iconst.i32 0
    v13 = load.i64 notrap aligned v0+8
    v14 = load.i64 notrap aligned v13+8
    v15 = call_indirect sig1, v14(v0, v9, v11, v1, v12)  ; v9 = 0, v12 = 0
    v16 = iconst.i32 0
    v17 = load.i64 notrap aligned v0+8
    v18 = load.i64 notrap aligned v17+16
    call_indirect sig2, v18(v0, v16, v8)  ; v16 = 0
    return v15, v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64) -> i64, i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4, v5 = call fn0(v0, v3)
    store notrap aligned v4, v2
    store notrap aligned v5, v2+8
    return
}
"#,
    );
}

/// Allocate and complete fixed and repeated uninitialized storage.
#[test]
fn test_emit_native_uninitialized_allocation() {
    let program = TestProgram::mir(
        r#"
export function allocate(v0: int64): slice<int32, unique, mutable> {
entry(v0: int64):
    v1: uninit<ref<int32, unique, mutable>> = new.uninit int32
    v2: ref<int32, unique, mutable> = new.complete v1
    v3: uninit<slice<int32, unique, mutable>> = new.slice.uninit int32, v0
    v4: slice<int32, unique, mutable> = new.complete v3
    free v2
    return v4
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64) -> i64, i64 native {
    gv0 = symbol colocated userextname0
    gv1 = symbol colocated userextname1
    sig0 = (i64, i32, i32, i32) -> i64 native
    sig1 = (i64, i32, i32, i64, i32) -> i64 native
    sig2 = (i64, i32, i64) native

block0(v0: i64, v1: i64):
    v2 = iconst.i32 0
    v3 = symbol_value.i64 gv0
    v4 = load.i32 notrap aligned v3
    v5 = iconst.i32 1
    v6 = load.i64 notrap aligned v0+8
    v7 = load.i64 notrap aligned v6
    v8 = call_indirect sig0, v7(v0, v2, v4, v5)  ; v2 = 0, v5 = 1
    v9 = iconst.i32 0
    v10 = symbol_value.i64 gv1
    v11 = load.i32 notrap aligned v10
    v12 = iconst.i32 1
    v13 = load.i64 notrap aligned v0+8
    v14 = load.i64 notrap aligned v13+8
    v15 = call_indirect sig1, v14(v0, v9, v11, v1, v12)  ; v9 = 0, v12 = 1
    v16 = iconst.i32 0
    v17 = load.i64 notrap aligned v0+8
    v18 = load.i64 notrap aligned v17+16
    call_indirect sig2, v18(v0, v16, v8)  ; v16 = 0
    return v15, v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64) -> i64, i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4, v5 = call fn0(v0, v3)
    store notrap aligned v4, v2
    store notrap aligned v5, v2+8
    return
}
"#,
    );
}

/// Allocate shared heap storage through the shared collector.
#[test]
fn test_emit_native_shared_allocation() {
    let program = TestProgram::mir(
        r#"
export function allocate(): ref<int32, unique, mutable, shared> {
entry:
    v0: ref<int32, unique, mutable, shared> = new.zeroed int32
    return v0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64) -> i64 native {
    gv0 = symbol colocated userextname0
    sig0 = (i64, i32, i32, i32) -> i64 native

block0(v0: i64):
    v1 = iconst.i32 1
    v2 = symbol_value.i64 gv0
    v3 = load.i32 notrap aligned v2
    v4 = iconst.i32 0
    v5 = load.i64 notrap aligned v0+8
    v6 = load.i64 notrap aligned v5
    v7 = call_indirect sig0, v6(v0, v1, v3, v4)  ; v1 = 1, v4 = 0
    return v7
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = call fn0(v0)
    store notrap aligned v3, v2
    return
}
"#,
    );
}
