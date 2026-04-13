use super::assert_format;

/// Formats allocation and deallocation operations canonically.
#[test]
fn test_format_allocation_family() {
    assert_format(
        r#"
function allocFamily(value0: int64): ref<int32, raw, addressSpace(stack)> {
entry0(value0: int64):
    value1: ref<int32, managed> = managed.alloc int32
    value2: ref<int32, managed> = managed.allocArray int32, value0
    value3: ref<int32, raw> = raw.alloc int32
    raw.free value3
    value4: ref<int32, raw, addressSpace(stack)> = stack.alloc int32
    return value4
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
    value1: ref<int32, raw, addressSpace(global)> = global.address counter
    value2: ref<int32, borrowed, addressSpace(stack)> = local.address local0
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

/// Formats atomic load, store, fence, and barrier operations canonically.
#[test]
fn test_format_atomic_load_store_fence_and_barrier_family() {
    assert_format(
        r#"
function atomics(value0: ref<int32, raw>): int32 {
entry0(value0: ref<int32, raw>):
    value1: int32 = atomic.load value0, acquire, device, device, [global, makeVisible]
    atomic.store value0, value1, release, device, device, global
    atomic.fence sequentiallyConsistent, device, device, any
    barrier workgroup, workgroup, any
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
function atomics(value0: ref<uint32, raw>): uint32 {
entry0(value0: ref<uint32, raw>):
    value1: uint32 = 1uint32
    value2: uint32 = 2uint32
    value3: (uint32, boolean) = atomic.cas value0, value1, value2, relaxed, device, device, any
    value4: uint32 = atomic.rmw.umin value0, value2, relaxed, device, device, any
    return value4
}
"#,
    );
}
