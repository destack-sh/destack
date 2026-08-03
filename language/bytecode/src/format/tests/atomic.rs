use super::assert_format_eq;

/// Format atomic ordering, scope, and storage operands canonically.
#[test]
fn test_format_atomic_operations() {
    assert_format_eq(
        r#"
function f0 {
    atomic.load r2, r0,acquire:uint32
atomic.store.pointer r0,r1,release:uint32
atomic.rmw.add r2, r0,r1,acquireRelease:uint32
atomic.cas r2,r3,r0,r1,r4,acquireRelease,failure(acquire):uint32
atomic.fence sequentiallyConsistent,scope(device),storage(shared)
return r2
}
"#,
        r#"
function f0 {
    atomic.load r2, r0, acquire: uint32
    atomic.store.pointer r0, r1, release: uint32
    atomic.rmw.add r2, r0, r1, acquireRelease: uint32
    atomic.cas r2, r3, r0, r1, r4, acquireRelease, failure(acquire): uint32
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    return r2
}
"#,
    );
}
