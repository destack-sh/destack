use super::compile_mir_to_normalized_clif;

/// Conditional branch selects between two target blocks.
/// The MIR branch becomes a brif instruction in CLIF.
#[test]
fn test_conditional_branch() {
    let mir = r#"
function select(v0: boolean): int32 {
bb0(v0: boolean):
    branch v0, bb1, bb2

bb1:
    v1: int32 = const 1int32
    return v1

bb2:
    v2: int32 = const 0int32
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int8): int32 native {
bb0(v0: int8):
    brif v0, bb1, bb2

bb1:
    v1 = const.int32 1
    return v1

bb2:
    v2 = const.int32 0
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
function jump_test(): int32 {

bb0:
    jump bb1

bb1:
    v0: int32 = const 42int32
    return v0
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(): int32 native {
bb0:
    jump bb1

bb1:
    v0 = const.int32 42
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
function phi_test(v0: boolean): int32 {
bb0(v0: boolean):
    branch v0, bb1, bb2
bb1:
    v1: int32 = const 10int32
    jump bb3(v1)
bb2:
    v2: int32 = const 20int32
    jump bb3(v2)
bb3(v3: int32):
    return v3
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int8): int32 native {
bb0(v0: int8):
    brif v0, bb1, bb2

bb1:
    v2 = const.int32 10
    jump bb3(v2)

bb2:
    v3 = const.int32 20
    jump bb3(v3)

bb3(v1: int32):
    return v1
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Switch uses Cranelift's Switch helper for efficient dispatch.
#[test]
fn test_switch_simple() {
    let mir = r#"
function dispatch(v0: int32): int32 {
bb0(v0: int32):
    switch v0, bb3, 0 => bb1, 1 => bb2
bb1:
    v1: int32 = const 100int32
    return v1
bb2:
    v2: int32 = const 200int32
    return v2
bb3:
    v3: int32 = const 0int32
    return v3
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32): int32 native {
bb0(v0: int32):
    br_table v0, bb3, [bb1, bb2]

bb1:
    v1 = const.int32 100
    return v1

bb2:
    v2 = const.int32 200
    return v2

bb3:
    v3 = const.int32 0
    return v3
}"#
    .trim();
    assert_eq!(clif, expected);
}

/// Values defined in one block can be used in later blocks.
#[test]
fn test_sequential_blocks() {
    let mir = r#"
function sequential(): int32 {
bb0:
    v0: int32 = const 1int32
    jump bb1
bb1:
    v1: int32 = const 2int32
    v2: int32 = int.add v0, v1
    jump bb2
bb2:
    v3: int32 = const 3int32
    v4: int32 = int.add v2, v3
    return v4
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(): int32 native {
bb0:
    v0 = const.int32 1
    jump bb1

bb1:
    v1 = const.int32 2
    v2 = int.add.int32 v0, v1
    jump bb2

bb2:
    v3 = const.int32 3
    v4 = int.add.int32 v2, v3
    return v4
}"#
    .trim();
    assert_eq!(clif, expected);
}
