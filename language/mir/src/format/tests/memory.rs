use super::{assert_format, assert_format_eq};

/// Formats allocation operations canonically.
#[test]
fn test_format_allocation_family() {
    assert_format(
        r#"
function allocFamily(v0: int64): ref<int32, raw, space(frame)> {
entry(v0: int64):
    v1: ref<int32, managed> = new.zeroed int32
    v2: slice<int32, managed> = new.slice.zeroed int32, v0
    v3: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: uninit<ref<int32, managed>> = new.uninit int32
    v6: ref<int32, managed> = new.complete v5
    v7: uninit<slice<int32, managed>> = new.slice.uninit int32, v0
    v8: slice<int32, managed> = new.complete v7
    v9: ref<int32, raw, space(frame)> = frame.alloc.uninit int32
    return v9
}
"#,
    );
}

/// Formats fallible allocation terminators canonically.
#[test]
fn test_format_fallible_allocation_family() {
    assert_format(
        r#"
function allocTry(v0: int64): int32 {
entry(v0: int64):
    new.zeroed.try int32 => b1, b2

b1(v1: ref<int32, managed>):
    new.slice.uninit.try int32, v0 => b3, b2

b2:
    v4: int32 = 0
    return v4

b3(v2: uninit<slice<int32, managed>>):
    v3: slice<int32, managed> = new.complete v2
    v5: int32 = 1
    return v5
}
"#,
    );
}

/// Formats slice descriptors canonically.
#[test]
fn test_format_slice_descriptor() {
    assert_format(
        r#"
function subslice(v0: slice<int32, borrowed, lifetime(0)>, v1: int64, v2: int64): slice<int32, borrowed, lifetime(0)> {
entry(v0: slice<int32, borrowed, lifetime(0)>, v1: int64, v2: int64):
    v3: slice<int32, borrowed, lifetime(0)> = slice v0, v1, v2
    return v3
}
"#,
    );
}

/// Formats load and store families canonically.
#[test]
fn test_format_load_store_family() {
    assert_format(
        r#"
global counter: int32 = zeroInit

function memory(v0: ref<int32, raw>): int32 {
    local l0: int32

entry(v0: ref<int32, raw>):
    v1: ref<int32, raw> = global.address counter
    v2: ref<int32, borrowed, space(frame)> = local.address l0
    v3: int32 = load v0
    store v0, v3
    local.set l0, v3
    v4: int32 = local.get l0
    v5: int32 = load v1
    store v2, v5
    return v4
}
"#,
    );
}

/// Formats atomic load, store, and fence operations canonically.
#[test]
fn test_format_atomic_load_store_and_fence_family() {
    assert_format(
        r#"
function atomics(v0: ref<atomic<int32>, raw, space(frame)>): int32 {
entry(v0: ref<atomic<int32>, raw, space(frame)>):
    v1: int32 = atomic.load v0, acquire, scope(device), volatile
    atomic.store v0, v1, release, scope(device)
    atomic.fence sequentiallyConsistent, scope(device), memory(device), [static, makeVisible]
    return v1
}
"#,
    );
}

/// Formats atomic compare-exchange and rmw operations canonically.
#[test]
fn test_format_atomic_compare_exchange_and_rmw_family() {
    assert_format(
        r#"
function atomics(v0: ref<atomic<uint32>, raw, space(frame)>): uint32 {
entry(v0: ref<atomic<uint32>, raw, space(frame)>):
    v1: uint32 = 1
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas v0, v1, v2, acquireRelease, failure(acquire)
    v4: uint32 = atomic.rmw.umin v0, v2, relaxed
    return v4
}
"#,
    );
}

/// Formats default compare-exchange failure ordering canonically.
#[test]
fn test_format_atomic_compare_exchange_default_failure_ordering() {
    assert_format_eq(
        r#"
function atomics(v0: ref<atomic<uint32>, raw, space(frame)>): (uint32, boolean) {
entry(v0: ref<atomic<uint32>, raw, space(frame)>):
    v1: uint32 = 1
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas v0, v1, v2, acquireRelease, failure(acquire)
    return v3
}
"#,
        r#"
function atomics(v0: ref<atomic<uint32>, raw, space(frame)>): (uint32, boolean) {
entry(v0: ref<atomic<uint32>, raw, space(frame)>):
    v1: uint32 = 1
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas v0, v1, v2, acquireRelease, failure(acquire)
    return v3
}
"#,
    );
}

/// Formats cleanup and pinning operations canonically.
#[test]
fn test_format_cleanup_and_pin_family() {
    assert_format(
        r#"
function cleanup(v0: ref<int32, managed>): void {
entry(v0: ref<int32, managed>):
    v1: ref<int32, managed> = pin v0
    unpin v1
    drop v0
    return
}
"#,
    );
}

/// Formats projected drops canonically.
#[test]
fn test_format_projected_drop() {
    assert_format(
        r#"
function cleanup(v0: slice<int32, unique>, v1: int64, v2: int64): void {
entry(v0: slice<int32, unique>, v1: int64, v2: int64):
    drop place(v0, slice(v1, v2))
    return
}
"#,
    );
}

/// Formats place projections canonically.
#[test]
fn test_format_place_projection_family() {
    assert_format(
        r#"
function cleanup(v0: slice<int32, unique>, v1: int64, v2: int64): void {
entry(v0: slice<int32, unique>, v1: int64, v2: int64):
    drop place(v0, field(0))
    drop place(v0, element(1))
    drop place(v0, element(any))
    drop place(v0, index(v1))
    drop place(v0, slice(v1, v2))
    drop place(v0, variant(7int32))
    return
}
"#,
    );
}
