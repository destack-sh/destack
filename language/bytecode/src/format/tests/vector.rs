use super::assert_format_eq;

/// Format vector operations with explicit lane types and counts.
#[test]
fn test_format_vector_operations() {
    assert_format_eq(
        r#"
function f0(): t0 {
    vector.splat.int32x4 r1:r2, r0
return r1:r2
}
"#,
        r#"
function f0(): t0 {
    vector.splat.int32x4 r1:r2, r0
    return r1:r2
}
"#,
    );
}

/// Format vector memory operations with their logical vector type.
#[test]
fn test_format_vector_memory() {
    assert_format_eq(
        r#"
function f0(): t0 {
    vector.load.int32x4 r1:r2, r0
vector.store.int32x4 r0,r1:r2
return
}
"#,
        r#"
function f0(): t0 {
    vector.load.int32x4 r1:r2, r0
    vector.store.int32x4 r0, r1:r2
    return
}
"#,
    );
}
