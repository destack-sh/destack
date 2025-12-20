//! Parser tests.

use crate::parse::Parser;
use crate::{MirFormatOptions, format_mir};

/// Test parsing and re-formatting produces the same output.
fn roundtrip(source: &str) {
    let (tree, strings) = Parser::parse(source).expect("parse failed");
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    assert_eq!(source.trim(), output.trim(), "roundtrip mismatch");
}

#[test]
fn test_roundtrip_simple_add() {
    roundtrip(
        r#"function @add(v0: i32, v1: i32) -> void {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_void_return() {
    roundtrip(
        r#"function @noop() -> void {
block0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_branch() {
    roundtrip(
        r#"function @select(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    jump block3(v1)
block2:
    v2 = iconst 0i32
    jump block3(v2)
block3(v3: i32):
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_call() {
    roundtrip(
        r#"extern function @callee(i32, i32) -> i32
function @caller() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = call @callee(v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_switch() {
    roundtrip(
        r#"function @dispatch(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block3, 0 => block1, 1 => block2
block1:
    v1 = iconst 100i32
    return v1
block2:
    v2 = iconst 200i32
    return v2
block3:
    v3 = iconst 0i32
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_managed_alloc() {
    roundtrip(
        r#"function @alloc_test() -> ref<i32> {
block0:
    v0 = managed.alloc i32
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_managed_alloc_array() {
    roundtrip(
        r#"function @array_alloc(v0: i64) -> ref<i32> {
block0(v0: i64):
    v1 = managed.alloc_array i32, v0
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_raw_alloc_and_free() {
    roundtrip(
        r#"function @raw_alloc() -> void {
block0:
    v0 = raw.alloc i32
    raw.free v0
    return
}"#,
    );
}

#[test]
fn test_roundtrip_stack_alloc() {
    roundtrip(
        r#"function @stack_alloc() -> rawptr<i32> {
block0:
    v0 = stack.alloc i32
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_nullable_ref() {
    roundtrip(
        r#"function @nullable_test() -> ref?<i32> {
block0:
    v0 = managed.alloc i32
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_string_constant() {
    roundtrip(
        r#"function @string_test() -> void {
block0:
    v0 = iconst "hello world"
    return
}"#,
    );
}

#[test]
fn test_roundtrip_string_with_escapes() {
    roundtrip(
        r#"function @escape_test() -> void {
block0:
    v0 = iconst "hello\nworld"
    return
}"#,
    );
}

#[test]
fn test_roundtrip_char_constant() {
    roundtrip(
        r#"function @char_test() -> void {
block0:
    v0 = iconst 'a'
    return
}"#,
    );
}

#[test]
fn test_roundtrip_char_escape() {
    roundtrip(
        r#"function @char_escape_test() -> void {
block0:
    v0 = iconst '\n'
    return
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_unary() {
    // unary intrinsic with one argument
    roundtrip(
        r#"function @sqrt_test(v0: f64) -> f64 {
block0(v0: f64):
    v1 = intrinsic.sqrt(v0)
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_binary() {
    // binary intrinsic with two arguments
    roundtrip(
        r#"function @min_test(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2 = intrinsic.min(v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_ternary() {
    // ternary intrinsic (fused multiply-add)
    roundtrip(
        r#"function @fma_test(v0: f64, v1: f64, v2: f64) -> f64 {
block0(v0: f64, v1: f64, v2: f64):
    v3 = intrinsic.fma(v0, v1, v2)
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_void() {
    // void intrinsic (no return value)
    roundtrip(
        r#"function @fence_test() -> void {
block0:
    intrinsic.atomic.fence()
    return
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_bit_manipulation() {
    // bit manipulation intrinsics
    roundtrip(
        r#"function @bit_test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = intrinsic.clz(v0)
    v2 = intrinsic.ctz(v1)
    v3 = intrinsic.popcnt(v2)
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_overflow() {
    // checked arithmetic intrinsics
    roundtrip(
        r#"function @add_overflow_test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = intrinsic.add.overflow(v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_atomic() {
    // atomic intrinsic with memory ordering
    roundtrip(
        r#"function @atomic_test(v0: rawptr<i32>) -> i32 {
block0(v0: rawptr<i32>):
    v1 = intrinsic.atomic.load(v0, acquire)
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_atomic_fence() {
    // atomic fence with ordering
    roundtrip(
        r#"function @fence_test() -> void {
block0:
    intrinsic.atomic.fence(seq_cst)
    return
}"#,
    );
}

#[test]
fn test_roundtrip_field_operations() {
    // field.get and field.set
    roundtrip(
        r#"function @field_test(v0: (i32, f64)) -> i32 {
block0(v0: (i32, f64)):
    v1 = field.get v0, 0
    v2 = iconst 42i32
    v3 = field.set v0, 0, v2
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_element_operations() {
    // element.get and element.set
    roundtrip(
        r#"function @element_test(v0: [i32; 10], v1: i64) -> i32 {
block0(v0: [i32; 10], v1: i64):
    v2 = element.get v0, v1
    v3 = iconst 42i32
    v4 = element.set v0, v1, v3
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_load_store() {
    // load and store through pointer
    roundtrip(
        r#"function @load_store_test(v0: rawptr<i32>) -> i32 {
block0(v0: rawptr<i32>):
    v1 = load v0
    v2 = iconst 42i32
    store v0, v2
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_unary_operations() {
    // unary operations
    roundtrip(
        r#"function @unary_test(v0: i32, v1: f64) -> i32 {
block0(v0: i32, v1: f64):
    v2 = ineg v0
    v3 = bnot v0
    v4 = fneg v1
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_cast_operations() {
    // cast operations
    roundtrip(
        r#"function @cast_test(v0: i32) -> i64 {
block0(v0: i32):
    v1 = sextend v0 -> i64
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_tuple_type() {
    // tuple type in function signature
    roundtrip(
        r#"function @tuple_test(v0: (i32, f64, bool)) -> (i32, f64, bool) {
block0(v0: (i32, f64, bool)):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_array_type() {
    // array type in function signature
    roundtrip(
        r#"function @array_test(v0: [i32; 10]) -> [i32; 10] {
block0(v0: [i32; 10]):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_function_pointer_type() {
    // function pointer type
    roundtrip(
        r#"function @fnptr_test(v0: fn(i32, i32) -> i64) -> fn(i32, i32) -> i64 {
block0(v0: fn(i32, i32) -> i64):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_struct_type() {
    // struct type in function signature
    roundtrip(
        r#"function @struct_test(v0: struct { i32, f64 }) -> struct { i32, f64 } {
block0(v0: struct { i32, f64 }):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_call_indirect() {
    // call through function pointer
    roundtrip(
        r#"function @indirect_call_test(v0: fn(i32) -> i32, v1: i32) -> i32 {
block0(v0: fn(i32) -> i32, v1: i32):
    v2 = call.indirect v0(v1)
    return v2
}"#,
    );
}
