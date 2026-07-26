use super::assert_format_eq;

/// Format strided slice views and physical length extraction.
#[test]
fn test_format_slice_operations() {
    assert_format_eq(
        r#"
function f0 {
    slice.view r3:r4, r0:r1,8,r2,r2
extract r5, r3:r4,8,8
return r3:r5
}
"#,
        r#"
function f0 {
    slice.view r3:r4, r0:r1, 8, r2, r2
    extract r5, r3:r4, 8, 8
    return r3:r5
}
"#,
    );
}
