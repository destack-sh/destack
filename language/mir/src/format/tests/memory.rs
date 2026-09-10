use super::{assert_format, assert_format_eq};

/// Formats allocation operations canonically.
#[test]
fn test_format_allocation_family() {
    assert_format(
        r#"
function allocFamily(v0: int64): ref<int32, managed, mutable, local> {
entry(v0: int64):
    v1: ref<int32, managed, mutable, local> = new.zeroed int32
    v2: slice<int32, managed, mutable, local> = new.slice.zeroed int32, v0
    v3: uninit<ref<int32, managed, mutable, local>> = new.uninit int32
    v4: ref<int32, managed, mutable, local> = new.complete v3
    v5: uninit<slice<int32, managed, mutable, local>> = new.slice.uninit int32, v0
    v6: slice<int32, managed, mutable, local> = new.complete v5
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

b1(v1: ref<int32, managed, mutable, local>):
    new.slice.uninit.try int32, v0 => b3 | b2

b2:
    v2: int32 = 0
    return v2

b3(v3: uninit<slice<int32, managed, mutable, local>>):
    v4: slice<int32, managed, mutable, local> = new.complete v3
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
function subslice<'a>(v0: slice<int32, borrowed, 'a & local, mutable>, v1: int64, v2: int64): slice<int32, borrowed, 'a & local, mutable> {
entry(v0: slice<int32, borrowed, 'a & local, mutable>, v1: int64, v2: int64):
    v3: slice<int32, borrowed, 'a & local, mutable> = slice.view v0, v1, v2
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
global counter: int32 = zeroinit

function memory<'a>(v0: ref<int32, borrowed, 'a & local, mutable>): int32 {
    local l0: int32

entry(v0: ref<int32, borrowed, 'a & local, mutable>):
    v1: ref<int32, borrowed, 'static & local, mutable> = global.address counter
    v2: ref<int32, borrowed, 'frame & frame, mutable> = local.address l0
    v3: int32 = load.copy v0
    v4: int32 = copy v3
    store v0, v3
    local.set l0, v4
    v5: int32 = local.get.copy l0
    v6: int32 = local.get l0
    v7: int32 = load.copy v1
    store v2, v7
    v8: int32 = load v2
    return v8
}
"#,
    );
}

/// Formats atomic load, store, and fence operations canonically.
#[test]
fn test_format_atomic_load_store_and_fence_family() {
    assert_format(
        r#"
function atomics<'a>(v0: ref<int32, borrowed, 'a & frame, mutable>): int32 {
entry(v0: ref<int32, borrowed, 'a & frame, mutable>):
    v1: int32 = atomic.load v0, acquire, scope(device)
    atomic.store v0, v1, release, scope(device)
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
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
function atomics<'a>(v0: ref<uint32, borrowed, 'a & frame, mutable>): uint32 {
entry(v0: ref<uint32, borrowed, 'a & frame, mutable>):
    v1: uint32 = 1
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas v0, v1, v2, acquireRelease, failure(acquire)
    v4: uint32 = atomic.rmw.min v0, v2, relaxed
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
function atomics<'a>(v0: ref<uint32, borrowed, 'a & frame, mutable>): (uint32, boolean) {
entry(v0: ref<uint32, borrowed, 'a & frame, mutable>):
    v1: uint32 = 1
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas v0, v1, v2, acquireRelease, failure(acquire)
    return v3
}
"#,
        r#"
function atomics<'a>(v0: ref<uint32, borrowed, 'a & frame, mutable>): (uint32, boolean) {
entry(v0: ref<uint32, borrowed, 'a & frame, mutable>):
    v1: uint32 = 1
    v2: uint32 = 2
    v3: (uint32, boolean) = atomic.cas v0, v1, v2, acquireRelease, failure(acquire)
    return v3
}
"#,
    );
}

/// Formats cleanup and hold operations canonically.
#[test]
fn test_format_cleanup_and_hold_family() {
    assert_format(
        r#"
function cleanup(v0: ref<int32, managed, mutable, local>, v1: ref<int32, managed, mutable, local>): void {
entry(v0: ref<int32, managed, mutable, local>, v1: ref<int32, managed, mutable, local>):
    drop v0
    return
}
"#,
    );
}
