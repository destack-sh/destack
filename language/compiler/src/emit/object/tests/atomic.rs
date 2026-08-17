use crate::tests::TestProgram;

/// Emit atomic load, store, compare exchange, update, and fence operations.
#[test]
fn test_emit_atomic_operations() {
    let program = TestProgram::mir(
        r#"
export function atomics(v0: ref<atomic<uint32>, borrowed, mutable, frame>): uint32 {
entry(v0: ref<atomic<uint32>, borrowed, mutable, frame>):
    v1: uint32 = atomic.load v0, acquire, scope(device)
    atomic.store v0, v1, release, scope(device)
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas v0, v1, v2, acquireRelease, failure(acquire)
    v4: uint32 = atomic.rmw.min v0, v2, relaxed
    atomic.fence sequentiallyConsistent, scope(device), storage(device)
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
    atomic.fence sequentiallyConsistent, scope(device), storage(device)
    return r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64) -> i32 native {
block0(v0: i64, v1: i64):
    v2 = atomic_load.i32 notrap aligned v1
    atomic_store notrap aligned v2, v1
    v3 = iconst.i32 2
    v4 = atomic_cas v1, v2, v3  ; v3 = 2
    v5 = icmp eq v4, v2
    v6 = atomic_rmw.i32 umin v1, v3  ; v3 = 2
    fence 
    return v6
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64) -> i32 native
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
export function atomics(
    v0: ref<atomic<float32>, borrowed, mutable, frame>,
    v1: float32,
): float32 {
entry(v0: ref<atomic<float32>, borrowed, mutable, frame>, v1: float32):
    v2: float32 = atomic.rmw.xchg v0, v1, relaxed
    v3: float32 = atomic.rmw.sub v0, v2, relaxed
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
function u0:0(i64, i64, f32) -> f32 native {
block0(v0: i64, v1: i64, v2: f32):
    v3 = bitcast.i32 v2
    v4 = atomic_rmw.i32 xchg v1, v3
    v5 = bitcast.f32 v4
    v6 = atomic_load.i32 notrap aligned v1
    jump block1(v6)

block1(v7: i32):
    v9 = bitcast.f32 v7
    v10 = fsub v9, v5
    v11 = bitcast.i32 v10
    v12 = atomic_cas v1, v7, v11
    v13 = icmp eq v12, v7
    brif v13, block2(v12), block1(v12)

block2(v8: i32):
    v14 = bitcast.f32 v8
    return v14
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, f32) -> f32 native
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
