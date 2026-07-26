use crate::{Comparison, FunctionId, Opcode, Result, Scalar, ScalarCheck};

use super::TestParser;

/// Resolve forward instruction labels into end-relative branch displacements.
#[test]
fn test_parse_branch_targets() {
    let object = TestParser::new(
        r#"
function f0 {    branch r0, b0, b1

b0:
    constant.boolean r1, true
    return r1

b1:
    constant.boolean r1, false
    return r1
}
"#,
    )
    .parse();
    let instructions = object
        .instructions(FunctionId(0))
        .expect("defined function")
        .collect::<Result<Vec<_>>>()
        .expect("valid instructions");

    // branch targets name their exact destination instructions
    assert_eq!(instructions[0].opcode(), Opcode::BRANCH);
    let operands = instructions[0].operand_bytes();
    let yes = i32::from_le_bytes(operands[2..6].try_into().expect("yes branch"));
    let no = i32::from_le_bytes(operands[6..10].try_into().expect("no branch"));
    let yes_byte_len = instructions[1].byte_len() + instructions[2].byte_len();

    assert_eq!(yes, 0);
    assert_eq!(no, yes_byte_len as i32);
    assert_eq!(
        instructions.last().expect("return").opcode(),
        Opcode::RETURN
    );
}

/// Parse checks, fused branches, and switch tables into exact opcodes.
#[test]
fn test_parse_checked_control_flow() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {    check.nonzero.int32 r0 else b3
    check.type r2, t0 else b3
    branch.lt.int32 r0, r1 => b0, b2

b0:
    switch r0 { 0 => b1, default => b2 }

b1:
    return r0

b2:
    return r1

b3:
    trap bounds
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::check(ScalarCheck::Nonzero, Scalar::Int32).expect("check opcode"),
            Opcode::CHECK_EXACT_TYPE,
            Opcode::branch(Comparison::LessThan, Scalar::Int32).expect("branch opcode"),
            Opcode::SWITCH,
            Opcode::RETURN,
            Opcode::RETURN,
            Opcode::TRAP,
        ]
    );
}

/// Parse panic values and panic propagation.
#[test]
fn test_parse_panic() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {    panic r0, t0
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(opcodes, vec![Opcode::PANIC_VALUE]);
}

/// Parse await and yield suspension with explicit continuation edges.
#[test]
fn test_parse_suspension() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {    await r2:r3, f1, r0 => b0 | b2 | b3

b0:
    yield r4:r5, r2:r3 => b1 | b3

b1:
    return r4:r5

b2:
    return

b3:
    unwind.resume
}

function f1 {    return
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::AWAIT,
            Opcode::YIELD,
            Opcode::RETURN,
            Opcode::RETURN,
            Opcode::UNWIND_RESUME,
        ]
    );
}
