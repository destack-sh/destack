use super::compile_mir_to_normalized_clif;

/// Stack allocate an i32.
#[test]
fn test_stack_allocate_i32() {
    let mir = r#"
function alloc_i32(): ref<int32, raw> {
bb0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    return v0
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // stack.alloc creates a stack slot and returns its address
    let expected = r#"
function u0:0(): int64 native {
    ss0 = explicit_slot 4

bb0:
    v0 = stack_addr.int64 ss0
    return v0
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Stack allocate an i64.
#[test]
fn test_stack_allocate_i64() {
    let mir = r#"
function alloc_i64(): ref<int64, raw> {
bb0:
    v0: ref<int64, raw, space(stack)> = stack.alloc int64
    return v0
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(): int64 native {
    ss0 = explicit_slot 8

bb0:
    v0 = stack_addr.int64 ss0
    return v0
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Stack allocate and use: allocate, store, load.
#[test]
fn test_stack_allocate_and_use() {
    let mir = r#"
function alloc_store_load(): int32 {
bb0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = const 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(): int32 native {
    ss0 = explicit_slot 4

bb0:
    v0 = stack_addr.int64 ss0
    v1 = const.int32 42
    store v1, v0
    v2 = load.int32 v0
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}
