use crate::{Comparison, FunctionId, Opcode, Result, Scalar, ScalarCheck};

use super::TestParser;

/// Resolve forward instruction labels into end-relative branch displacements.
#[test]
fn test_parse_branch_targets() {
    let object = TestParser::new(
        r#"
function f0 {
    branch r0 => b0 | b1

b0:
    constant r1, true: boolean
    return r1

b1:
    constant r1, false: boolean
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
function f0 {
    check.nonzero r0: int32 | b3
    check.shift r0, 32: int32 | b3
    check.narrow r0: int64 -> int32 | b3
    check.add.overflow r0, r1: int32 | b3
    check.sub.overflow r0, r1: int32 | b3
    check.mul.overflow r0, r1: int32 | b3
    check.bounds r0, r1: uint64 | b3
    check.range r0, r1, r2: uint64 | b3
    poll
    check.type r2, t0 | b3
    branch.lt r0, r1: int32 => b0 | b2

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
            Opcode::check(ScalarCheck::Shift, Scalar::Int32).expect("check opcode"),
            Opcode::check(ScalarCheck::Narrow, Scalar::Int64).expect("check opcode"),
            Opcode::check(ScalarCheck::AddOverflow, Scalar::Int32).expect("check opcode"),
            Opcode::check(ScalarCheck::SubtractOverflow, Scalar::Int32).expect("check opcode"),
            Opcode::check(ScalarCheck::MultiplyOverflow, Scalar::Int32).expect("check opcode"),
            Opcode::check(ScalarCheck::Bounds, Scalar::Uint64).expect("check opcode"),
            Opcode::check(ScalarCheck::Range, Scalar::Uint64).expect("check opcode"),
            Opcode::POLL,
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
function f0 {
    panic r0, t0
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(opcodes, vec![Opcode::PANIC_VALUE]);
}
