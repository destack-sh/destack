use crate::tests::TestProgram;

/// Lower scalar atomic access directly to native atomic instructions.
#[test]
fn test_emit_native_atomic_operations() {
    let program = TestProgram::mir(
        r#"
export function atomics(v0: ref<atomic<uint32>, borrowed, mutable, frame>): uint32 {
entry(v0: ref<atomic<uint32>, borrowed, mutable, frame>):
    v1: uint32 = atomic.load v0, acquire, scope(device)
    atomic.store v0, v1, release, scope(device)
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas v0, v1, v2, acquireRelease, failure(acquire)
    v4: uint32 = atomic.rmw.umin v0, v2, relaxed
    atomic.fence sequentiallyConsistent, scope(device), storage(device)
    return v4
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
