//! Parser tests.

use crate::{format_mir, parse::Parser};

/// Test parsing and re-formatting produces the same output.
fn roundtrip(source: &str) {
    let tree = Parser::parse(source).expect("parse failed");
    let output = format_mir(&tree);
    assert_eq!(source.trim(), output.trim(), "roundtrip mismatch");
}

#[test]
fn test_roundtrip_simple_add() {
    roundtrip(
        r#"function @function0(v0: i32, v1: i32) -> void {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_void_return() {
    roundtrip(
        r#"function @function0() -> void {
block0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_branch() {
    roundtrip(
        r#"function @function0(v0: bool) -> i32 {
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

