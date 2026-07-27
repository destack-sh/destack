use super::assert_format_eq;

/// Format pointer materialization and calculation canonically.
#[test]
fn test_format_pointer_operations() {
    assert_format_eq(
        r#"
function f0 {
    frame.address r2, r0:r1
pointer.frame r3,r2
global.address r4,g0
pointer.global r5,r4
pointer.local r6,r2
pointer.shared r7,r3
pointer.add r8, r5,16
pointer.add r9, r3,r0
pointer.add r10, r3,r0,8
pointer.byteOffsetFrom r11,r10,r8
return r11
}
"#,
        r#"
function f0 {
    frame.address r2, r0:r1
    pointer.frame r3, r2
    global.address r4, g0
    pointer.global r5, r4
    pointer.local r6, r2
    pointer.shared r7, r3
    pointer.add r8, r5, 16
    pointer.add r9, r3, r0
    pointer.add r10, r3, r0, 8
    pointer.byteOffsetFrom r11, r10, r8
    return r11
}
"#,
    );
}
