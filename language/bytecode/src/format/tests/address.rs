use super::assert_format_eq;

/// Format stable addresses and both arithmetic representations canonically.
#[test]
fn test_format_address_operations() {
    assert_format_eq(
        r#"
function f0 {
    frame.address r2, r0:r1
global.address.constant r3,g0
global.address.local r4,g1
global.address.shared r5,g2
reference.add r6, r3,16
reference.add r7, r2,r0
reference.add r8, r2,r0,8
reference.diff r9,r8,r6
pointer.add r10,r1,16
pointer.diff r11,r10,r1
return r9
}
"#,
        r#"
function f0 {
    frame.address r2, r0:r1
    global.address.constant r3, g0
    global.address.local r4, g1
    global.address.shared r5, g2
    reference.add r6, r3, 16
    reference.add r7, r2, r0
    reference.add r8, r2, r0, 8
    reference.diff r9, r8, r6
    pointer.add r10, r1, 16
    pointer.diff r11, r10, r1
    return r9
}
"#,
    );
}
