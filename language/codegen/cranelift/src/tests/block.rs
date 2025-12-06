use super::compile_mir_to_normalized_clif;

/// Conditional branch selects between two target blocks.
/// The MIR branch becomes a brif instruction in CLIF.
#[test]
fn test_conditional_branch() {
    let mir = r#"
function @select(v0: bool) -> i32 {
block0:
    branch v0, block1, block2

block1:
    v1 = iconst 1i32
    return v1

block2:
    v2 = iconst 0i32
    return v2
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i8) -> i32 native {
block0(v0: i8):
    brif v0, block1, block2

block1:
    v1 = iconst.i32 1
    return v1

block2:
    v2 = iconst.i32 0
    return v2
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Unconditional jump transfers control to a single target.
/// The MIR jump becomes a CLIF jump instruction.
#[test]
fn test_unconditional_jump() {
    let mir = r#"
function @jump_test() -> i32 {

block0:
    jump block1

block1:
    v0 = iconst 42i32
    return v0
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0() -> i32 native {
block0:
    jump block1

block1:
    v0 = iconst.i32 42
    return v0
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Block parameters implement phi nodes for control flow merges.
/// Values are passed as arguments when jumping to a block with parameters.
/// Cranelift may renumber values, so v1/v2 in MIR may become v2/v3 in CLIF.
#[test]
fn test_block_parameters() {
    let mir = r#"
function @phi_test(v0: bool) -> i32 {
block0:
    branch v0, block1, block2
block1:
    v1 = iconst 10i32
    jump block3(v1)
block2:
    v2 = iconst 20i32
    jump block3(v2)
block3(v3: i32):
    return v3
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i8) -> i32 native {
block0(v0: i8):
    brif v0, block1, block2

block1:
    v2 = iconst.i32 10
    jump block3(v2)

block2:
    v3 = iconst.i32 20
    jump block3(v3)

block3(v1: i32):
    return v1
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Switch uses Cranelift's Switch helper for efficient dispatch.
#[test]
fn test_switch_simple() {
    let mir = r#"
function @dispatch(v0: i32) -> i32 {
block0:
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
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32) -> i32 native {
block0(v0: i32):
    br_table v0, block3, [block1, block2]

block1:
    v1 = iconst.i32 100
    return v1

block2:
    v2 = iconst.i32 200
    return v2

block3:
    v3 = iconst.i32 0
    return v3
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Values defined in one block can be used in later blocks.
#[test]
fn test_sequential_blocks() {
    let mir = r#"
function @sequential() -> i32 {
block0:
    v0 = iconst 1i32
    jump block1
block1:
    v1 = iconst 2i32
    v2 = iadd v0, v1
    jump block2
block2:
    v3 = iconst 3i32
    v4 = iadd v2, v3
    return v4
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0() -> i32 native {
block0:
    v0 = iconst.i32 1
    jump block1

block1:
    v1 = iconst.i32 2
    v2 = iadd.i32 v0, v1
    jump block2

block2:
    v3 = iconst.i32 3
    v4 = iadd.i32 v2, v3
    return v4
}"#
    .trim();
    assert_eq!(clif, expected);
}
