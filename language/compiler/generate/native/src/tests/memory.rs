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
function read_ptr(v0: ref<int32, raw, readonly>): int32 {
b0(v0: ref<int32, raw, readonly>):
    v1: int32 = load v0
    return v1
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int64): int32 native {
b0(v0: int64):
    v1 = load.int32 v0
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
function read_ptr_ptr(v0: ref<ref<int32, raw>, raw, readonly>): int32 {
b0(v0: ref<ref<int32, raw>, raw, readonly>):
    v1: ref<int32, raw, readonly> = load v0
    v2: int32 = load v1
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int64): int32 native {
b0(v0: int64):
    v1 = load.int64 v0
    v2 = load.int32 v1
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
function read_ptr64(v0: ref<int64, raw, readonly>): int64 {
b0(v0: ref<int64, raw, readonly>):
    v1: int64 = load v0
    return v1
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int64): int64 native {
b0(v0: int64):
    v1 = load.int64 v0
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
function read_bool(v0: ref<boolean, raw, readonly>): boolean {
b0(v0: ref<boolean, raw, readonly>):
    v1: boolean = load v0
    return v1
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int64): int8 native {
b0(v0: int64):
    v1 = load.int8 v0
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
function local_test(v0: int32): int32 {
    local local0: int32

b0(v0: int32):
    v1: int32 = local.get local0
    v2: int32 = int.add v0, v1
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32): int32 native {
    ss0 = explicit_slot 4

b0(v0: int32):
    v1 = stack_load.int32 ss0
    v2 = int.add v0, v1
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
function multi_local_test(v0: int32): int64 {
    local local0: int32
    local local1: int64

b0(v0: int32):
    v1: int32 = local.get local0
    v2: int64 = cast.extend.u v1 to int64
    v3: int64 = local.get local1
    v4: int64 = int.add v2, v3
    return v4
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32): int64 native {
    ss0 = explicit_slot 4
    ss1 = explicit_slot 8

b0(v0: int32):
    v1 = stack_load.int32 ss0
    v2 = cast.extend.u.int64 v1
    v3 = stack_load.int64 ss1
    v4 = int.add v2, v3
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
function read_float(v0: ref<float32, raw, readonly>): float32 {
b0(v0: ref<float32, raw, readonly>):
    v1: float32 = load v0
    return v1
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int64): float32 native {
b0(v0: int64):
    v1 = load.float32 v0
    return v1
}"#
    .trim();
    assert_eq!(clif, expected);
}
