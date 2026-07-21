use crate::{Comparison, FunctionId, Opcode, Result, Scalar, ScalarCheck, ValueType};

use super::TestParser;

/// Resolve forward instruction labels into end-relative branch displacements.
#[test]
fn test_parse_branch_targets() {
    let object = TestParser::new(
        r#"
export function choose(r0: boolean): boolean {
    branch r0, l0, l1

l0:
    r1: boolean = true
    return r1

l1:
    r1: boolean = false
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
type User

export function choose(r0: int32, r1: int32, r2: typeId): int32 {
    check.nonzero.int32 r0 else l3
    check.type r2: User else l3
    branch.lt.int32 r0, r1 => l0, l2

l0:
    switch r0 { 0 => l1, default => l2 }

l1:
    return r0

l2:
    return r1

l3:
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

/// Parse continuation edges, caught panic values, and panic propagation.
#[test]
fn test_parse_suspension_and_panic() {
    let object = TestParser::new(
        r#"
export function suspend(r0: int32) resume(int32): int32 {
    r1: int32 = yield r0 => l0 | l1

l0:
    return r1

l1:
    unwind.resume
}

export function fail(): void {
    r0: ref<managed, space(local)> = catch
    panic r0
}
"#,
    )
    .parse();
    let suspend = object
        .instructions(FunctionId(0))
        .expect("suspending function")
        .map(|instruction| instruction.expect("valid instruction").opcode())
        .collect::<Vec<_>>();
    let fail = object
        .instructions(FunctionId(1))
        .expect("failing function")
        .map(|instruction| instruction.expect("valid instruction").opcode())
        .collect::<Vec<_>>();

    assert_eq!(
        suspend,
        vec![Opcode::YIELD, Opcode::RETURN, Opcode::UNWIND_RESUME]
    );
    let function = object.function(FunctionId(0)).expect("suspending function");
    assert_eq!(
        function.resume_parameters(object.value_types()),
        &[ValueType::scalar(Scalar::Int32)]
    );
    for (operation, opcode) in suspend.iter().copied().enumerate() {
        let instruction = object
            .instruction(FunctionId(0), operation as u32)
            .expect("valid instruction")
            .expect("operation instruction");
        assert_eq!(instruction.opcode(), opcode);
    }
    assert_eq!(fail, vec![Opcode::CATCH, Opcode::PANIC_VALUE]);
}
