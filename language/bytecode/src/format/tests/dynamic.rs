use super::assert_format_eq;

/// Format dynamic binding, field access, and runtime type projection canonically.
#[test]
fn test_format_dynamic_operations() {
    assert_format_eq(
        r#"
function f0 {
dynamic.bind r1:r2,r0,d0
dynamic.read r3,r1:r2[1],8
dynamic.type r4,r1:r2
return r1:r4
}
"#,
        r#"
function f0 {
    dynamic.bind r1:r2, r0, d0
    dynamic.read r3, r1:r2[1], 8
    dynamic.type r4, r1:r2
    return r1:r4
}
"#,
    );
}
