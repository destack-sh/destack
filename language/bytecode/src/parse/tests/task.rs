use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse completed, eager, parked, cancelled, and detached tasks.
#[test]
fn test_parse_tasks() {
    let (_, opcodes) = TestParser::new(
        r#"
function tasks {
    task.resolve r1, r0, t0
    task.start r3, r2
    task.park r3, r4
    task.cancel r3
    task.detach r3
    return
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::TASK_RESOLVE,
            Opcode::TASK_START,
            Opcode::TASK_PARK,
            Opcode::TASK_CANCEL,
            Opcode::TASK_DETACH,
            Opcode::RETURN,
        ]
    );
}
