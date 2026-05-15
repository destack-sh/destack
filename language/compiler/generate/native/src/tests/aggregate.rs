use super::compile_mir_to_normalized_clif;

/// Construct a struct and return its address (as a pointer).
#[test]
fn test_struct_construction_i32_i32() {
    // aggregates in cranelift are always pointers, so return ref type
    let mir = r#"
function make_point(): ref<{ int32, int32 }, raw> {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: { int32, int32 } = struct { int32, int32 } (v0, v1)
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // struct construction: allocate stack slot, store each field, return address
    let expected = r#"
function u0:0(): int64 native {
    ss0 = explicit_slot 8, align = 4

b0:
    v0 = const.int32 10
    v1 = const.int32 20
    v2 = stack_addr.int64 ss0
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
function make_pair(): ref<(int32, int32), raw> {
b0:
    v0: int32 = 42int32
    v1: int32 = 99int32
    v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // tuple construction: allocate stack slot, store each element, return address
    let expected = r#"
function u0:0(): int64 native {
    ss0 = explicit_slot 8, align = 4

b0:
    v0 = const.int32 42
    v1 = const.int32 99
    v2 = stack_addr.int64 ss0
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
function make_array(): ref<[int32; 3], raw> {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: [int32; 3] = array [int32; 3] (v0, v1, v2)
    return v3
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // array construction: allocate stack slot, store each element at index*size
    let expected = r#"
function u0:0(): int64 native {
    ss0 = explicit_slot 12, align = 4

b0:
    v0 = const.int32 1
    v1 = const.int32 2
    v2 = const.int32 3
    v3 = stack_addr.int64 ss0
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
function make_mixed(): ref<{ int8, int32, int16 }, raw> {
b0:
    v0: int8 = 1int8
    v1: int32 = 100int32
    v2: int16 = 50int16
    v3: { int8, int32, int16 } = struct { int8, int32, int16 } (v0, v1, v2)
    return v3
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // layout: i8 at 0, i32 at 4 (aligned), i16 at 8
    // total size: 10, padded to alignment 4 -> 12
    let expected = r#"
function u0:0(): int64 native {
    ss0 = explicit_slot 12, align = 4

b0:
    v0 = const.int8 1
    v1 = const.int32 100
    v2 = const.int16 50
    v3 = stack_addr.int64 ss0
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
function make_array(): ref<[int64; 2], raw> {
b0:
    v0: int64 = 100int64
    v1: int64 = 200int64
    v2: [int64; 2] = array [int64; 2] (v0, v1)
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // array: 2 * 8 = 16 bytes, 8-byte alignment
    let expected = r#"
function u0:0(): int64 native {
    ss0 = explicit_slot 16, align = 8

b0:
    v0 = const.int64 100
    v1 = const.int64 200
    v2 = stack_addr.int64 ss0
    store v0, v2
    store v1, v2+8
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}
