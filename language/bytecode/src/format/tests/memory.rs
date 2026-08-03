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
store r5,r8:r9,12
load r8:r9,r5,12
memory.copy r0,r1,r2
memory.move r0,r1,16
memory.fill r0,r3,r2
memory.compare r4,r0,r1,16
prefetch.read r1
return r7
}
"#,
        r#"
function f0 {
    frame.address r5, r4
    store r5, r3: int32
    load r7, r5: int32
    store r5, r8:r9, 12
    load r8:r9, r5, 12
    memory.copy r0, r1, r2
    memory.move r0, r1, 16
    memory.fill r0, r3, r2
    memory.compare r4, r0, r1, 16
    prefetch.read r1
    return r7
}
"#,
    );
}
