use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse queueing and cancelling runtime waiters.
#[test]
fn test_parse_waiters() {
    let (_, queue_opcodes) = TestParser::new(
        r#"
function settle {
    waiter.queue r0, r1, t1
    return
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    let (_, cancel_opcodes) = TestParser::new(
        r#"
function cancel {
    waiter.cancel r0
    return
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(queue_opcodes, vec![Opcode::WAITER_QUEUE, Opcode::RETURN]);
    assert_eq!(cancel_opcodes, vec![Opcode::WAITER_CANCEL, Opcode::RETURN]);
}
