use super::assert_format_eq;

/// Format atomic ordering, scope, and storage operands canonically.
#[test]
fn test_format_atomic_operations() {
    assert_format_eq(
        r#"
export function exchange(r0:address,r1:uint32):uint32{
r2:uint32=atomic.load.uint32 r0,acquire
atomic.store.uint32 r0,r1,release
r2:uint32=atomic.rmw.add.uint32 r0,r1,acquireRelease
atomic.fence sequentiallyConsistent,scope(device),storage(shared)
return r2
}
"#,
        r#"
export function exchange(r0: address, r1: uint32): uint32 {
    r2: uint32 = atomic.load.uint32 r0, acquire
    atomic.store.uint32 r0, r1, release
    r2: uint32 = atomic.rmw.add.uint32 r0, r1, acquireRelease
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    return r2
}
"#,
    );
}
