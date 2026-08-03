use super::{assert_format, assert_format_eq};

/// Formats allocation operations canonically.
#[test]
fn test_format_allocation_family() {
    assert_format(
        r#"
function allocFamily(v0: int64): ref<int32, managed, mutable> {
entry(v0: int64):
    v1: ref<int32, managed, mutable> = new.zeroed int32
    v2: slice<int32, managed, mutable> = new.slice.zeroed int32, v0
    v3: uninit<ref<int32, managed, mutable>> = new.uninit int32
    v4: ref<int32, managed, mutable> = new.complete v3
    v5: uninit<slice<int32, managed, mutable>> = new.slice.uninit int32, v0
    v6: slice<int32, managed, mutable> = new.complete v5
    return v4
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
    new.zeroed.try int32 => b1 | b2

b1(v1: ref<int32, managed, mutable>):
    new.slice.uninit.try int32, v0 => b3 | b2

b2:
    v2: int32 = 0
    return v2

b3(v3: uninit<slice<int32, managed, mutable>>):
    v4: slice<int32, managed, mutable> = new.complete v3
    v5: int32 = 1
    return v5
}
"#,
    );
}

/// Formats slice views canonically.
#[test]
fn test_format_slice_view() {
    assert_format(
        r#"
function subslice<'L0>(v0: slice<int32, borrowed, 'L0, mutable>, v1: int64, v2: int64): slice<int32, borrowed, 'L0, mutable> {
entry(v0: slice<int32, borrowed, 'L0, mutable>, v1: int64, v2: int64):
    v3: slice<int32, borrowed, 'L0, mutable> = slice.view v0, v1, v2
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

function memory(v0: ref<int32, borrowed, mutable>): int32 {
    local l0: int32

entry(v0: ref<int32, borrowed, mutable>):
    v1: ref<int32, borrowed, mutable> = global.address counter
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
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
function atomics(v0: ref<atomic<int32>, borrowed, mutable, frame>): int32 {
entry(v0: ref<atomic<int32>, borrowed, mutable, frame>):
    v1: int32 = atomic.load v0, acquire, scope(device)
    atomic.store v0, v1, release, scope(device)
    atomic.fence sequentiallyConsistent, scope(device), storage(device)
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
function atomics(v0: ref<atomic<uint32>, borrowed, mutable, frame>): uint32 {
entry(v0: ref<atomic<uint32>, borrowed, mutable, frame>):
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
function atomics(v0: ref<atomic<uint32>, borrowed, mutable, frame>): (uint32, boolean) {
entry(v0: ref<atomic<uint32>, borrowed, mutable, frame>):
    v1: uint32 = 1
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas v0, v1, v2, acquireRelease, failure(acquire)
    return v3
}
"#,
        r#"
function atomics(v0: ref<atomic<uint32>, borrowed, mutable, frame>): (uint32, boolean) {
entry(v0: ref<atomic<uint32>, borrowed, mutable, frame>):
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
function cleanup(v0: ref<int32, managed, mutable>): void {
entry(v0: ref<int32, managed, mutable>):
    v1: ref<int32, managed, mutable> = pin v0
    unpin v1
    drop v0
    return
}
"#,
    );
}
