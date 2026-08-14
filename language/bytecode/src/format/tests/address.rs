use super::assert_format_eq;

/// Format stable addresses and both arithmetic representations canonically.
#[test]
fn test_format_address_operations() {
    assert_format_eq(
        r#"
function f0 {
    frame.address r2, r0:r1
global.address r3,g0
global.address r4,g1
global.address r5,g2
address.add r6, r3,16
address.add r7, r2,r0
address.add r8, r2,r0,8
address.diff r9,r8,r6
address.add r10,r1,16
address.diff r11,r10,r1
return r9
}
"#,
        r#"
function f0 {
    frame.address r2, r0:r1
    global.address r3, g0
    global.address r4, g1
    global.address r5, g2
    address.add r6, r3, 16
    address.add r7, r2, r0
    address.add r8, r2, r0, 8
    address.diff r9, r8, r6
    address.add r10, r1, 16
    address.diff r11, r10, r1
    return r9
}
"#,
    );
}
