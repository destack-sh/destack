use super::assert_format_eq;

/// Format pointer materialization and calculation canonically.
#[test]
fn test_format_pointer_operations() {
    assert_format_eq(
        r#"
function f0 {
    pointer.frame r3, r0:r1
pointer.global r4,g0
pointer.local r5,r2
pointer.shared r6,r3
pointer.add r7, r4,16
pointer.add r8, r3,r0
pointer.add r9, r3,r0,8
pointer.byteOffsetFrom r10,r9,r7
return r10
}
"#,
        r#"
function f0 {
    pointer.frame r3, r0:r1
    pointer.global r4, g0
    pointer.local r5, r2
    pointer.shared r6, r3
    pointer.add r7, r4, 16
    pointer.add r8, r3, r0
    pointer.add r9, r3, r0, 8
    pointer.byteOffsetFrom r10, r9, r7
    return r10
}
"#,
    );
}
