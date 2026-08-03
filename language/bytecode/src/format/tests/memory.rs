use super::assert_format_eq;

/// Format scalar, packed, and ranged memory operations canonically.
#[test]
fn test_format_memory() {
    assert_format_eq(
        r#"
function f0 {
    frame.address r5, r4
store r5,r3: int32
load r7, r5: int32
load.constant r6,r4:uint64
store.pointer r11,r1:uint16
load.volatile r10,r5:uint32
store.volatile.pointer r11,r10:uint32
store r5,r8:r9,12
load.pointer r8:r9,r11,12
store.volatile r5,r8:r9,12
load.volatile.pointer r8:r9,r11,12
memory.copy r0,r1,r2
memory.move.pointer.memory r0,r1,16
memory.fill.pointer r0,r3,r2
memory.compare.constant.pointer r4,r0,r1,16
prefetch.read.pointer r1
return r7
}
"#,
        r#"
function f0 {
    frame.address r5, r4
    store r5, r3: int32
    load r7, r5: int32
    load.constant r6, r4: uint64
    store.pointer r11, r1: uint16
    load.volatile r10, r5: uint32
    store.volatile.pointer r11, r10: uint32
    store r5, r8:r9, 12
    load.pointer r8:r9, r11, 12
    store.volatile r5, r8:r9, 12
    load.volatile.pointer r8:r9, r11, 12
    memory.copy r0, r1, r2
    memory.move.pointer.memory r0, r1, 16
    memory.fill.pointer r0, r3, r2
    memory.compare.constant.pointer r4, r0, r1, 16
    prefetch.read.pointer r1
    return r7
}
"#,
    );
}
