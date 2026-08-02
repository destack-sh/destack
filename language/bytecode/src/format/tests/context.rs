use super::assert_format_eq;

/// Format execution context operations canonically.
#[test]
fn test_format_context_operations() {
    assert_format_eq(
        r#"
function scope {
context.current r3
context.bind r4,r3,r0,r1:r2,a0,16
context.replace r5,r4
context.get r6:r7,r4,r0,r1:r2,16
context.replace r8,r5
return r6:r7
}
"#,
        r#"
function scope {
    context.current r3
    context.bind r4, r3, r0, r1:r2, a0, 16
    context.replace r5, r4
    context.get r6:r7, r4, r0, r1:r2, 16
    context.replace r8, r5
    return r6:r7
}
"#,
    );
}
