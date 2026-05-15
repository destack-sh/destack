use super::compile_mir_to_normalized_clif;

/// Void functions produce no return value.
#[test]
fn test_empty_void_function() {
    let mir = r#"
function empty(): void {
b0:
    return
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0() native {
b0:
    return
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Function parameters become entry block parameters in Cranelift.
#[test]
fn test_function_with_i32_params() {
    let mir = r#"
function add(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32, int32): int32 native {
b0(v0: int32, v1: int32):
    v2 = int.add v0, v1
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Different integer types are preserved in the signature and constants.
#[test]
fn test_function_returning_i64() {
    let mir = r#"
function returns_i64(): int64 {
b0:
    v0: int64 = 42int64
    return v0
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(): int64 native {
b0:
    v0 = const.int64 42
    return v0
}"#
    .trim();
    assert_eq!(clif, expected);
}
