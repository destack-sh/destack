use crate::tests::TestProgram;

/// Emit atomic load, store, compare exchange, update, and fence operations.
#[test]
fn test_emit_bytecode_atomic_operations() {
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

    program.assert_bytecode(
        r#"
function atomics {
    atomic.load r1, r0, acquire, scope(device): uint32
    atomic.store r0, r1, release, scope(device): uint32
    constant r2, 2: uint32
    atomic.cas r4, r5, r0, r1, r2, acquireRelease, failure(acquire): uint32
    aggregate r3, [r4 @ 0:4, r5 @ 4:1]
    atomic.rmw.min r1, r0, r2, relaxed: uint32
    atomic.fence sequentiallyConsistent, scope(device), storage(device)
    return r1
}
"#,
    );
}
