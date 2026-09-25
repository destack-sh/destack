use crate::tests::TestProgram;

/// Emit fixed and repeated initialized and uninitialized allocations.
#[test]
fn test_emit_allocation_forms() {
    let program = TestProgram::mir(
        r#"
export function allocate(v0: int64): slice<int32, unique, mutable> {
entry(v0: int64):
    v1: ref<int32, unique, mutable> = new.zeroed int32, local
    v2: uninit<ref<int32, unique, mutable>> = new.uninit int32, local
    v3: ref<int32, unique, mutable> = new.complete v2
    v4: slice<int32, unique, mutable> = new.slice.zeroed int32, v0, local
    v5: uninit<slice<int32, unique, mutable>> = new.slice.uninit int32, v0, local
    v6: slice<int32, unique, mutable> = new.complete v5
    return v6
}
"#,
    );

    program.assert_bytecode(
        r#"
function allocate {
    new.zeroed r1, a0
    new.uninit r1, a1
    move r2, r1
    new.slice.zeroed r1:r2, a2, r0
    new.slice.uninit r1:r2, a3, r0
    move r3:r4, r1:r2
    return r3:r4
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64) -> i64, i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    gv3 = symbol colocated userextname1
    gv4 = symbol colocated userextname2
    gv5 = symbol colocated userextname3
    sig0 = (i64, i32, i32, i32) -> i64 native
    sig1 = (i64, i32, i32, i32) -> i64 native
    sig2 = (i64, i32, i32, i64, i32) -> i64 native
    sig3 = (i64, i32, i32, i64, i32) -> i64 native
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = iconst.i32 0
    v3 = symbol_value.i64 gv2
    v4 = load.i32 notrap aligned v3
    v5 = iconst.i32 0
    v6 = load.i64 notrap aligned region0 v0+8
    v7 = load.i64 notrap aligned v6
    v8 = call_indirect sig0, v7(v0, v2, v4, v5)  ; v2 = 0, v5 = 0
    v9 = iconst.i32 0
    v10 = symbol_value.i64 gv3
    v11 = load.i32 notrap aligned v10
    v12 = iconst.i32 1
    v13 = load.i64 notrap aligned region0 v0+8
    v14 = load.i64 notrap aligned v13
    v15 = call_indirect sig1, v14(v0, v9, v11, v12)  ; v9 = 0, v12 = 1
    v16 = iconst.i32 0
    v17 = symbol_value.i64 gv4
    v18 = load.i32 notrap aligned v17
    v19 = iconst.i32 0
    v20 = load.i64 notrap aligned region0 v0+8
    v21 = load.i64 notrap aligned v20+8
    v22 = call_indirect sig2, v21(v0, v16, v18, v1, v19)  ; v16 = 0, v19 = 0
    v23 = iconst.i32 0
    v24 = symbol_value.i64 gv5
    v25 = load.i32 notrap aligned v24
    v26 = iconst.i32 1
    v27 = load.i64 notrap aligned region0 v0+8
    v28 = load.i64 notrap aligned v27+8
    v29 = call_indirect sig3, v28(v0, v23, v25, v1, v26)  ; v23 = 0, v26 = 1
    return v29, v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i64, i64 native
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

/// Allocate fixed and repeated managed heap storage through the runtime ABI.
#[test]
fn test_emit_allocation_and_free() {
    let program = TestProgram::mir(
        r#"
export function allocate(v0: int64): slice<int32, unique, mutable> {
entry(v0: int64):
    v1: ref<int32, unique, mutable> = new.zeroed int32, local
    v2: slice<int32, unique, mutable> = new.slice.zeroed int32, v0, local
    release v1
    return v2
}
"#,
    );

    program.assert_bytecode(
        r#"
function allocate {
    new.zeroed r1, a0
    new.slice.zeroed r2:r3, a1, r0
    free r1
    return r2:r3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64) -> i64, i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    gv3 = symbol colocated userextname1
    sig0 = (i64, i32, i32, i32) -> i64 native
    sig1 = (i64, i32, i32, i64, i32) -> i64 native
    sig2 = (i64, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = iconst.i32 0
    v3 = symbol_value.i64 gv2
    v4 = load.i32 notrap aligned v3
    v5 = iconst.i32 0
    v6 = load.i64 notrap aligned region0 v0+8
    v7 = load.i64 notrap aligned v6
    v8 = call_indirect sig0, v7(v0, v2, v4, v5)  ; v2 = 0, v5 = 0
    v9 = iconst.i32 0
    v10 = symbol_value.i64 gv3
    v11 = load.i32 notrap aligned v10
    v12 = iconst.i32 0
    v13 = load.i64 notrap aligned region0 v0+8
    v14 = load.i64 notrap aligned v13+8
    v15 = call_indirect sig1, v14(v0, v9, v11, v1, v12)  ; v9 = 0, v12 = 0
    v16 = load.i64 notrap aligned region0 v0+8
    v17 = load.i64 notrap aligned v16+24
    call_indirect sig2, v17(v0, v8)
    return v15, v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i64, i64 native
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
fn test_emit_uninitialized_allocation() {
    let program = TestProgram::mir(
        r#"
export function allocate(v0: int64): slice<int32, unique, mutable> {
entry(v0: int64):
    v1: uninit<ref<int32, unique, mutable>> = new.uninit int32, local
    v2: ref<int32, unique, mutable> = new.complete v1
    v3: uninit<slice<int32, unique, mutable>> = new.slice.uninit int32, v0, local
    v4: slice<int32, unique, mutable> = new.complete v3
    release v2
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function allocate {
    new.uninit r1, a0
    move r2, r1
    new.slice.uninit r3:r4, a1, r0
    move r0:r1, r3:r4
    free r2
    return r0:r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64) -> i64, i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    gv3 = symbol colocated userextname1
    sig0 = (i64, i32, i32, i32) -> i64 native
    sig1 = (i64, i32, i32, i64, i32) -> i64 native
    sig2 = (i64, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = iconst.i32 0
    v3 = symbol_value.i64 gv2
    v4 = load.i32 notrap aligned v3
    v5 = iconst.i32 1
    v6 = load.i64 notrap aligned region0 v0+8
    v7 = load.i64 notrap aligned v6
    v8 = call_indirect sig0, v7(v0, v2, v4, v5)  ; v2 = 0, v5 = 1
    v9 = iconst.i32 0
    v10 = symbol_value.i64 gv3
    v11 = load.i32 notrap aligned v10
    v12 = iconst.i32 1
    v13 = load.i64 notrap aligned region0 v0+8
    v14 = load.i64 notrap aligned v13+8
    v15 = call_indirect sig1, v14(v0, v9, v11, v1, v12)  ; v9 = 0, v12 = 1
    v16 = load.i64 notrap aligned region0 v0+8
    v17 = load.i64 notrap aligned v16+24
    call_indirect sig2, v17(v0, v8)
    return v15, v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i64, i64 native
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
fn test_emit_shared_allocation() {
    let program = TestProgram::mir(
        r#"
export function allocate(): ref<int32, unique, mutable> {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, shared
    return v0
}
"#,
    );

    program.assert_bytecode(
        r#"
function allocate {
    new.zeroed r0, a0
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    sig0 = (i64, i32, i32, i32) -> i64 native
    stack_limit = gv1

block0(v0: i64):
    v1 = iconst.i32 1
    v2 = symbol_value.i64 gv2
    v3 = load.i32 notrap aligned v2
    v4 = iconst.i32 0
    v5 = load.i64 notrap aligned region0 v0+8
    v6 = load.i64 notrap aligned v5
    v7 = call_indirect sig0, v6(v0, v1, v3, v4)  ; v1 = 1, v4 = 0
    return v7
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = call fn0(v0)
    store notrap aligned v3, v2
    return
}
"#,
    );
}

/// Emit one heap release, which destroys the values of an erased unique allocation.
#[test]
fn test_emit_release_of_an_erased_unique_value() {
    let program = TestProgram::mir(
        r#"
type Writer { }

export function release(v0: dynamic<Writer, unique, mutable>): void {
entry(v0: dynamic<Writer, unique, mutable>):
    release v0
    return
}

export function releaseFunction(v0: function<() => void, once, unique, mutable>): void {
entry(v0: function<() => void, once, unique, mutable>):
    release v0
    return
}
"#,
    );

    program.assert_bytecode(
        r#"
function release {
    release r0
    return
}

function releaseFunction {
    release r1
    return
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i32) native {
    ss0 = explicit_slot 1, key = 0
    ss1 = explicit_slot 16, align = 8, key = 1
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    sig0 = (i64, i64, i32, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    v3 = stack_addr.i64 ss0
    v4 = stack_addr.i64 ss1
    store notrap aligned region1 v1, v4
    store notrap aligned region1 v2, v4+8
    v5 = symbol_value.i64 gv2
    v6 = load.i32 notrap aligned v5
    v7 = load.i64 notrap aligned region0 v0+8
    v8 = load.i64 notrap aligned v7+16
    call_indirect sig0, v8(v0, v1, v6, v3), stack_map=[i8 @ ss0+0, i8 @ ss1+0]
    return
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i32) native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    call fn0(v0, v3, v4)
    return
}

function u0:2(i64 vmctx, i64, i64) native {
    ss0 = explicit_slot 1, key = 4294967296
    ss1 = explicit_slot 16, align = 8, key = 4294967297
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    sig0 = (i64, i64, i32, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64):
    v3 = stack_addr.i64 ss0
    v4 = stack_addr.i64 ss1
    store notrap aligned region1 v1, v4
    store notrap aligned region1 v2, v4+8
    v5 = symbol_value.i64 gv2
    v6 = load.i32 notrap aligned v5
    v7 = load.i64 notrap aligned region0 v0+8
    v8 = load.i64 notrap aligned v7+16
    call_indirect sig0, v8(v0, v2, v6, v3), stack_map=[i8 @ ss0+0, i8 @ ss1+0]
    return
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64) native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    call fn0(v0, v3, v4)
    return
}
"#,
    );
}
