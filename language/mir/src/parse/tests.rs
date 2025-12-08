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
fn test_roundtrip_managed_allocate() {
    roundtrip(
        r#"function @alloc_test() -> ref<i32> {
block0:
    v0 = managed_allocate i32
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_managed_allocate_array() {
    roundtrip(
        r#"function @array_alloc(v0: i64) -> ref<i32> {
block0(v0: i64):
    v1 = managed_allocate_array i32, v0
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_raw_allocate_and_free() {
    roundtrip(
        r#"function @raw_alloc() -> void {
block0:
    v0 = raw_allocate i32
    raw_free v0
    return
}"#,
    );
}

#[test]
fn test_roundtrip_stack_allocate() {
    roundtrip(
        r#"function @stack_alloc() -> rawptr<i32> {
block0:
    v0 = stack_allocate i32
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_drop() {
    roundtrip(
        r#"function @drop_test(v0: ref<i32>) -> void {
block0(v0: ref<i32>):
    drop v0
    return
}"#,
    );
}

#[test]
fn test_roundtrip_nullable_ref() {
    roundtrip(
        r#"function @nullable_test() -> ref?<i32> {
block0:
    v0 = managed_allocate i32
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
