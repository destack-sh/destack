use super::compile_mir_to_normalized_clif;

/// Construct a struct and return its address (as a pointer).
#[test]
fn test_struct_construction_i32_i32() {
    // aggregates in cranelift are always pointers, so return ref type
    let mir = r#"
function @make_point() -> ref<raw { i32, i32 }> {
block0:
    v0: i32 = iconst 10i32
    v1: i32 = iconst 20i32
    v2: { i32, i32 } = struct { i32, i32 } (v0, v1)
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // struct construction: allocate stack slot, store each field, return address
    let expected = r#"
function u0:0() -> i64 native {
    ss0 = explicit_slot 8, align = 4

block0:
    v0 = iconst.i32 10
    v1 = iconst.i32 20
    v2 = stack_addr.i64 ss0
    store v0, v2
    store v1, v2+4
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Construct a tuple and return its address.
#[test]
fn test_tuple_construction_i32_i32() {
    let mir = r#"
function @make_pair() -> ref<raw (i32, i32)> {
block0:
    v0: i32 = iconst 42i32
    v1: i32 = iconst 99i32
    v2: (i32, i32) = tuple (i32, i32) (v0, v1)
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // tuple construction: allocate stack slot, store each element, return address
    let expected = r#"
function u0:0() -> i64 native {
    ss0 = explicit_slot 8, align = 4

block0:
    v0 = iconst.i32 42
    v1 = iconst.i32 99
    v2 = stack_addr.i64 ss0
    store v0, v2
    store v1, v2+4
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Construct an array of 3 i32 elements and return its address.
#[test]
fn test_array_construction_i32_3() {
    let mir = r#"
function @make_array() -> ref<raw [i32; 3]> {
block0:
    v0: i32 = iconst 1i32
    v1: i32 = iconst 2i32
    v2: i32 = iconst 3i32
    v3: [i32; 3] = array [i32; 3] (v0, v1, v2)
    return v3
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // array construction: allocate stack slot, store each element at index*size
    let expected = r#"
function u0:0() -> i64 native {
    ss0 = explicit_slot 12, align = 4

block0:
    v0 = iconst.i32 1
    v1 = iconst.i32 2
    v2 = iconst.i32 3
    v3 = stack_addr.i64 ss0
    store v0, v3
    store v1, v3+4
    store v2, v3+8
    return v3
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Construct a struct with mixed field sizes to test alignment.
#[test]
fn test_struct_construction_mixed_types() {
    let mir = r#"
function @make_mixed() -> ref<raw { i8, i32, i16 }> {
block0:
    v0: i8 = iconst 1i8
    v1: i32 = iconst 100i32
    v2: i16 = iconst 50i16
    v3: { i8, i32, i16 } = struct { i8, i32, i16 } (v0, v1, v2)
    return v3
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // layout: i8 at 0, i32 at 4 (aligned), i16 at 8
    // total size: 10, padded to alignment 4 -> 12
    let expected = r#"
function u0:0() -> i64 native {
    ss0 = explicit_slot 12, align = 4

block0:
    v0 = iconst.i8 1
    v1 = iconst.i32 100
    v2 = iconst.i16 50
    v3 = stack_addr.i64 ss0
    store v0, v3
    store v1, v3+4
    store v2, v3+8
    return v3
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Construct an array of i64 elements and return its address.
#[test]
fn test_array_construction_i64_2() {
    let mir = r#"
function @make_array() -> ref<raw [i64; 2]> {
block0:
    v0: i64 = iconst 100i64
    v1: i64 = iconst 200i64
    v2: [i64; 2] = array [i64; 2] (v0, v1)
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // array: 2 * 8 = 16 bytes, 8-byte alignment
    let expected = r#"
function u0:0() -> i64 native {
    ss0 = explicit_slot 16, align = 8

block0:
    v0 = iconst.i64 100
    v1 = iconst.i64 200
    v2 = stack_addr.i64 ss0
    store v0, v2
    store v1, v2+8
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}
