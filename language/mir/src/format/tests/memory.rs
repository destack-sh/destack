use super::{assert_format, assert_format_eq};

/// Formats allocation and deallocation operations canonically.
#[test]
fn test_format_allocation_family() {
    assert_format(
        r#"
function allocFamily(value0: int64): ref<int32, raw, space(stack)> {
entry0(value0: int64):
    value1: ref<int32, managed> = new int32
    value2: slice<int32, managed> = new.slice int32, value0
    value3: ref<int32, raw> = raw.alloc int32
    raw.free value3
    value4: ref<int32, raw, space(stack)> = stack.alloc int32
    return value4
}
"#,
    );
}

/// Formats slice descriptors canonically.
#[test]
fn test_format_slice_descriptor() {
    assert_format(
        r#"
function subslice(value0: slice<int32, borrowed, lifetime(0)>, value1: int64, value2: int64): slice<int32, borrowed, lifetime(0)> {
entry0(value0: slice<int32, borrowed, lifetime(0)>, value1: int64, value2: int64):
    value3: slice<int32, borrowed, lifetime(0)> = slice value0, value1, value2
    return value3
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

function memory(value0: ref<int32, raw>): int32 {
    local local0: int32, owned

entry0(value0: ref<int32, raw>):
    value1: ref<int32, raw> = global.address counter
    value2: ref<int32, borrowed, space(frame)> = local.address local0
    value3: int32 = load value0
    store value0, value3
    local.set local0, value3
    value4: int32 = local.get local0
    value5: int32 = load value1
    store value2, value5
    return value4
}
"#,
    );
}

/// Formats atomic load, store, and fence operations canonically.
#[test]
fn test_format_atomic_load_store_and_fence_family() {
    assert_format(
        r#"
function atomics(value0: ref<atomic<int32>, raw>): int32 {
entry0(value0: ref<atomic<int32>, raw>):
    value1: int32 = atomic.load value0, acquire, scope(device), volatile
    atomic.store value0, value1, release, scope(device)
    atomic.fence sequentiallyConsistent, scope(device), memory(device), [static, makeVisible]
    return value1
}
"#,
    );
}

/// Formats atomic compare-exchange and rmw operations canonically.
#[test]
fn test_format_atomic_compare_exchange_and_rmw_family() {
    assert_format(
        r#"
function atomics(value0: ref<atomic<uint32>, raw>): uint32 {
entry0(value0: ref<atomic<uint32>, raw>):
    value1: uint32 = 1uint32
    value2: uint32 = 2uint32
    value3: (uint32, boolean) = atomic.cas value0, value1, value2, acquireRelease, failure(acquire)
    value4: uint32 = atomic.rmw.umin value0, value2, relaxed
    return value4
}
"#,
    );
}

/// Formats default compare-exchange failure ordering canonically.
#[test]
fn test_format_atomic_compare_exchange_default_failure_ordering() {
    assert_format_eq(
        r#"
function atomics(value0: ref<atomic<uint32>, raw>): (uint32, boolean) {
entry0(value0: ref<atomic<uint32>, raw>):
    value1: uint32 = 1uint32
    value2: uint32 = 2uint32
    value3: (uint32, boolean) = atomic.cas value0, value1, value2, acquireRelease
    return value3
}
"#,
        r#"
function atomics(value0: ref<atomic<uint32>, raw>): (uint32, boolean) {
entry0(value0: ref<atomic<uint32>, raw>):
    value1: uint32 = 1uint32
    value2: uint32 = 2uint32
    value3: (uint32, boolean) = atomic.cas value0, value1, value2, acquireRelease, failure(acquire)
    return value3
}
"#,
    );
}

/// Formats cleanup and pinning operations canonically.
#[test]
fn test_format_cleanup_and_pin_family() {
    assert_format(
        r#"
function cleanup(value0: ref<int32, managed>): void {
entry0(value0: ref<int32, managed>):
    value1: ref<int32, managed> = pin value0
    unpin value1
    drop value0
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
function cleanup(value0: slice<int32, unique>, value1: int64, value2: int64): void {
entry0(value0: slice<int32, unique>, value1: int64, value2: int64):
    drop place(value0, range(value1, value2))
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
function cleanup(value0: slice<int32, unique>, value1: int64, value2: int64): void {
entry0(value0: slice<int32, unique>, value1: int64, value2: int64):
    drop place(value0, field(0))
    drop place(value0, element(1))
    drop place(value0, index(value1))
    drop place(value0, range(value1, value2))
    return
}
"#,
    );
}
