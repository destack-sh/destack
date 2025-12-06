use super::compile_mir_to_normalized_clif;

/// Void functions produce no return value.
#[test]
fn test_empty_void_function() {
    let mir = r#"
function @empty() -> void {
block0:
    return
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0() native {
block0:
    return
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Function parameters become entry block parameters in Cranelift.
#[test]
fn test_function_with_i32_params() {
    let mir = r#"
function @add(v0: i32, v1: i32) -> i32 {
block0:
    v2 = iadd v0, v1
    return v2
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32, i32) -> i32 native {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Different integer types are preserved in the signature and constants.
#[test]
fn test_function_returning_i64() {
    let mir = r#"
function @returns_i64() -> i64 {
block0:
    v0 = iconst 42i64
    return v0
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0() -> i64 native {
block0:
    v0 = iconst.i64 42
    return v0
}"#
    .trim();
    assert_eq!(clif, expected);
}
