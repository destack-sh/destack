use super::assert_format_eq;

/// Format multiline function bodies and frame addresses.
#[test]
fn test_format_function() {
    assert_format_eq(
        r#"
function f0 {
    frame.address r1, r0
    pointer.frame r2, r1
move r3, r0
return r3
}
"#,
        r#"
function f0 {
    frame.address r1, r0
    pointer.frame r2, r1
    move r3, r0
    return r3
}
"#,
    );
}
