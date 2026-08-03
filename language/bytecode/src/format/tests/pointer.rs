use super::assert_format_eq;

/// Format frame and global addresses with pointer calculations canonically.
#[test]
fn test_format_pointer_operations() {
    assert_format_eq(
        r#"
function f0 {
    frame.address r2, r0:r1
global.address.constant r3,g0
global.address.local r4,g1
global.address.shared r5,g2
pointer.add r6, r3,16
pointer.add r7, r2,r0
pointer.add r8, r2,r0,8
pointer.diff r9,r8,r6
return r9
}
"#,
        r#"
function f0 {
    frame.address r2, r0:r1
    global.address.constant r3, g0
    global.address.local r4, g1
    global.address.shared r5, g2
    pointer.add r6, r3, 16
    pointer.add r7, r2, r0
    pointer.add r8, r2, r0, 8
    pointer.diff r9, r8, r6
    return r9
}
"#,
    );
}
