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
    intrinsic.atomic_fence()
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
    v2 = intrinsic.add_overflow(v0, v1)
    return v2
}"#,
    );
}
