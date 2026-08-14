use super::assert_format_eq;

/// Format scalar, packed, and ranged memory operations canonically.
#[test]
fn test_format_memory() {
    assert_format_eq(
        r#"
function f0 {
    frame.address r5, r4
store.int32 r5,r3
load.int32 r7, r5
load.uint64 r6,r4
store.uint16 pointer r11,r1
load.volatile.uint32 r10,r5
store.volatile.uint32 pointer r11,r10
memory.store r5,r8:r9,12
memory.load r8:r9,pointer r11,12
memory.store.volatile r5,r8:r9,12
memory.load.volatile r8:r9,pointer r11,12
memory.copy r0,r1,r2
memory.move pointer r0,r1,16
memory.fill pointer r0,r3,r2
memory.compare r4,r0,pointer r1,16
prefetch.read pointer r1
return r7
}
"#,
        r#"
function f0 {
    frame.address r5, r4
    store.int32 r5, r3
    load.int32 r7, r5
    load.uint64 r6, r4
    store.uint16 pointer r11, r1
    load.volatile.uint32 r10, r5
    store.volatile.uint32 pointer r11, r10
    memory.store r5, r8:r9, 12
    memory.load r8:r9, pointer r11, 12
    memory.store.volatile r5, r8:r9, 12
    memory.load.volatile r8:r9, pointer r11, 12
    memory.copy r0, r1, r2
    memory.move pointer r0, r1, 16
    memory.fill pointer r0, r3, r2
    memory.compare r4, r0, pointer r1, 16
    prefetch.read pointer r1
    return r7
}
"#,
    );
}
