use super::compile_mir_to_normalized_clif;

/// Stack allocate an i32.
#[test]
fn test_stack_allocate_i32() {
    let mir = r#"
function @alloc_i32() -> rawptr<i32> {
block0:
    v0 = stack_allocate i32
    return v0
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // stack_allocate creates a stack slot and returns its address
    let expected = r#"
function u0:0() -> i64 native {
    ss0 = explicit_slot 4

block0:
    v0 = stack_addr.i64 ss0
    return v0
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Stack allocate an i64.
#[test]
fn test_stack_allocate_i64() {
    let mir = r#"
function @alloc_i64() -> rawptr<i64> {
block0:
    v0 = stack_allocate i64
    return v0
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0() -> i64 native {
    ss0 = explicit_slot 8

block0:
    v0 = stack_addr.i64 ss0
    return v0
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Stack allocate and use: allocate, store, load.
#[test]
fn test_stack_allocate_and_use() {
    let mir = r#"
function @alloc_store_load() -> i32 {
block0:
    v0 = stack_allocate i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    return v2
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0() -> i32 native {
    ss0 = explicit_slot 4

block0:
    v0 = stack_addr.i64 ss0
    v1 = iconst.i32 42
    store v1, v0
    v2 = load.i32 v0
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}
