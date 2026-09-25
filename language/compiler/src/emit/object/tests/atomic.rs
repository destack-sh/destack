use crate::tests::TestProgram;

/// Emit atomic load, store, compare exchange, update, and fence operations.
#[test]
fn test_emit_atomic_operations() {
    let program = TestProgram::mir(
        r#"
export function atomics<'a>(v0: ref<uint32, borrowed, 'a, mutable>): uint32 {
entry(v0: ref<uint32, borrowed, 'a, mutable>):
    v1: uint32 = atomic.load (*v0), acquire, scope(device)
    atomic.store (*v0), v1, release, scope(device)
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas (*v0), v1, v2, acquireRelease, failure(acquire)
    v4: uint32 = atomic.rmw.min (*v0), v2, relaxed
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function atomics {
    atomic.load.uint32 r1, r0, acquire, scope(device)
    atomic.store.uint32 r0, r1, release, scope(device)
    constant.uint32 r2, 2
    atomic.cas.uint32 r4, r5, r0, r1, r2, acquireRelease, failure(acquire)
    aggregate r3, [r4 @ 0:4, r5 @ 4:1]
    atomic.rmw.min.uint32 r1, r0, r2, relaxed
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    return r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64):
    v2 = load.i64 notrap aligned region0 v0+40
    v3 = iadd v2, v1
    v4 = atomic_load.i32 notrap aligned v3
    v5 = load.i64 notrap aligned region0 v0+40
    v6 = iadd v5, v1
    atomic_store notrap aligned v4, v6
    v7 = iconst.i32 2
    v8 = load.i64 notrap aligned region0 v0+40
    v9 = iadd v8, v1
    v10 = atomic_cas v9, v4, v7  ; v7 = 2
    v11 = icmp eq v10, v4
    v12 = load.i64 notrap aligned region0 v0+40
    v13 = iadd v12, v1
    v14 = atomic_rmw.i32 umin v13, v7  ; v7 = 2
    fence 
    return v14
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}

/// Select floating-point exchange and arithmetic from the atomic value type.
#[test]
fn test_emit_float_atomic_operations() {
    let program = TestProgram::mir(
        r#"
export function atomics<'a>(v0: ref<float32, borrowed, 'a, mutable>, v1: float32): float32 {
entry(v0: ref<float32, borrowed, 'a, mutable>, v1: float32):
    v2: float32 = atomic.rmw.xchg (*v0), v1, relaxed
    v3: float32 = atomic.rmw.sub (*v0), v2, relaxed
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function atomics {
    atomic.exchange.float32 r2, r0, r1, relaxed
    atomic.rmw.sub.float32 r1, r0, r2, relaxed
    return r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, f32) -> f32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: f32):
    v3 = load.i64 notrap aligned region0 v0+40
    v4 = iadd v3, v1
    v5 = bitcast.i32 v2
    v6 = atomic_rmw.i32 xchg v4, v5
    v7 = bitcast.f32 v6
    v8 = load.i64 notrap aligned region0 v0+40
    v9 = iadd v8, v1
    v10 = atomic_load.i32 notrap aligned v9
    jump block1(v10)

block1(v11: i32):
    v13 = bitcast.f32 v11
    v14 = fsub v13, v7
    v15 = bitcast.i32 v14
    v16 = atomic_cas v9, v11, v15
    v17 = icmp eq v16, v11
    brif v17, block2(v16), block1(v16)

block2(v12: i32):
    v18 = bitcast.f32 v12
    return v18
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, f32) -> f32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.f32 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Apply atomic operations through a projected field place.
#[test]
fn test_emit_projected_atomic_operations() {
    let program = TestProgram::mir(
        r#"
type Counter {
    label: int64;
    count: uint32;
}

export function bump<'a>(v0: ref<Counter, borrowed, 'a, mutable>, v1: uint32): uint32 {
entry(v0: ref<Counter, borrowed, 'a, mutable>, v1: uint32):
    v2: uint32 = atomic.load (*v0).1, acquire
    atomic.store (*v0).1, v1, release
    v3: uint32 = atomic.rmw.add (*v0).1, v1, relaxed
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function bump {
    address.add r3, r0, 8
    atomic.load.uint32 r2, r3, acquire
    address.add r3, r0, 8
    atomic.store.uint32 r3, r1, release
    address.add r3, r0, 8
    atomic.rmw.add.uint32 r2, r3, r1, relaxed
    return r2
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i32):
    v3 = iconst.i64 8
    v4 = iadd v1, v3  ; v3 = 8
    v5 = load.i64 notrap aligned region0 v0+40
    v6 = iadd v5, v4
    v7 = atomic_load.i32 notrap aligned v6
    v8 = iconst.i64 8
    v9 = iadd v1, v8  ; v8 = 8
    v10 = load.i64 notrap aligned region0 v0+40
    v11 = iadd v10, v9
    atomic_store notrap aligned v2, v11
    v12 = iconst.i64 8
    v13 = iadd v1, v12  ; v12 = 8
    v14 = load.i64 notrap aligned region0 v0+40
    v15 = iadd v14, v13
    v16 = atomic_rmw.i32 add v15, v2
    return v16
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}
