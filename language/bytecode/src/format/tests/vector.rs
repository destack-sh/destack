use super::assert_format_eq;

/// Format vector operations with explicit lane types and counts.
#[test]
fn test_format_vector_operations() {
    assert_format_eq(
        r#"
function f0 {
    vector.splat r1:r2, r0: vector<int32, 4>
return r1:r2
}
"#,
        r#"
function f0 {
    vector.splat r1:r2, r0: vector<int32, 4>
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
function f0 {
    vector.load r1:r2, r0:vector<int32,4>
vector.store r0,r1:r2:vector<int32,4>
return
}
"#,
        r#"
function f0 {
    vector.load r1:r2, r0: vector<int32, 4>
    vector.store r0, r1:r2: vector<int32, 4>
    return
}
"#,
    );
}
