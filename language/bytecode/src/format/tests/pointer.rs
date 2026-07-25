use super::assert_format_eq;

/// Format global, register, additive, and distance pointer operations canonically.
#[test]
fn test_format_pointer_operations() {
    assert_format_eq(
        r#"
function f0(): t0 {
    global.address r3, g0
address r4, r1
pointer.add r5, r3,16
pointer.add r6, r4,r0
pointer.add r7, r4,r0,8
pointer.distance r8, r7,r5
return r8
}
"#,
        r#"
function f0(): t0 {
    global.address r3, g0
    address r4, r1
    pointer.add r5, r3, 16
    pointer.add r6, r4, r0
    pointer.add r7, r4, r0, 8
    pointer.distance r8, r7, r5
    return r8
}
"#,
    );
}
