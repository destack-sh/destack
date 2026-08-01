use super::assert_format_eq;

/// Format pointer materialization and calculation canonically.
#[test]
fn test_format_pointer_operations() {
    assert_format_eq(
        r#"
function f0 {
    frame.address r2, r0:r1
pointer.frame r3,r2
global.address.constant r4,g0
global.address.local r5,g1
global.address.shared r6,g2
pointer.constant r7,r4
pointer.memory r8,r5
pointer.add r9, r7,16
pointer.add r10, r3,r0
pointer.add r11, r3,r0,8
pointer.byteOffsetFrom r12,r11,r9
return r12
}
"#,
        r#"
function f0 {
    frame.address r2, r0:r1
    pointer.frame r3, r2
    global.address.constant r4, g0
    global.address.local r5, g1
    global.address.shared r6, g2
    pointer.constant r7, r4
    pointer.memory r8, r5
    pointer.add r9, r7, 16
    pointer.add r10, r3, r0
    pointer.add r11, r3, r0, 8
    pointer.byteOffsetFrom r12, r11, r9
    return r12
}
"#,
    );
}
