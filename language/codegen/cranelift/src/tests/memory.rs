//! Memory operation tests.
//!
//! Tests for lowering MIR memory operations (load, store, locals)
//! to Cranelift IR.

use super::compile_mir_to_normalized_clif;

/// Load from a pointer dereferences memory.
/// The type of the loaded value is inferred from the pointer's pointee type.
#[test]
fn test_load_from_pointer() {
    let mir = r#"
function @read_ptr(v0: rawptr<i32>) -> i32 {
block0:
    v1 = load v0
    return v1
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i64) -> i32 native {
block0(v0: i64):
    v1 = load.i32 v0
    return v1
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Load from a pointer to pointer (double indirection).
/// The first load produces a pointer, the second load produces the final value.
#[test]
fn test_load_double_indirection() {
    let mir = r#"
function @read_ptr_ptr(v0: rawptr<rawptr<i32>>) -> i32 {
block0:
    v1 = load v0
    v2 = load v1
    return v2
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i64) -> i32 native {
block0(v0: i64):
    v1 = load.i64 v0
    v2 = load.i32 v1
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Load from a pointer to i64.
/// Verifies different pointed-to sizes are handled correctly.
#[test]
fn test_load_i64() {
    let mir = r#"
function @read_ptr64(v0: rawptr<i64>) -> i64 {
block0:
    v1 = load v0
    return v1
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i64) -> i64 native {
block0(v0: i64):
    v1 = load.i64 v0
    return v1
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Load a boolean from a pointer.
/// Booleans are i8 in Cranelift.
#[test]
fn test_load_bool() {
    let mir = r#"
function @read_bool(v0: rawptr<bool>) -> bool {
block0:
    v1 = load v0
    return v1
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i64) -> i8 native {
block0(v0: i64):
    v1 = load.i8 v0
    return v1
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Load from local variable produces the local's type.
/// Stack slots are used for mutable local variables.
#[test]
fn test_local_get() {
    let mir = r#"
function @local_test(v0: i32) -> i32 {
    local0: i32

block0:
    v1 = local.get local0
    v2 = iadd v0, v1
    return v2
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32) -> i32 native {
    ss0 = explicit_slot 4

block0(v0: i32):
    v1 = stack_load.i32 ss0
    v2 = iadd v0, v1
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Multiple local variables with different types.
/// Each gets its own stack slot with appropriate size.
#[test]
fn test_multiple_locals_load() {
    let mir = r#"
function @multi_local_test(v0: i32) -> i64 {
    local0: i32
    local1: i64

block0:
    v1 = local.get local0
    v2 = uextend v1 -> i64
    v3 = local.get local1
    v4 = iadd v2, v3
    return v4
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32) -> i64 native {
    ss0 = explicit_slot 4
    ss1 = explicit_slot 8

block0(v0: i32):
    v1 = stack_load.i32 ss0
    v2 = uextend.i64 v1
    v3 = stack_load.i64 ss1
    v4 = iadd v2, v3
    return v4
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Load float from pointer.
/// Float types use f32/f64 in Cranelift, not i32/i64.
#[test]
fn test_load_float() {
    let mir = r#"
function @read_float(v0: rawptr<f32>) -> f32 {
block0:
    v1 = load v0
    return v1
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i64) -> f32 native {
block0(v0: i64):
    v1 = load.f32 v0
    return v1
}"#
    .trim();
    assert_eq!(clif, expected);
}
