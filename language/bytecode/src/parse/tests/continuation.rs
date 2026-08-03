use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse synchronous and receiver-driven continuation ownership.
#[test]
fn test_parse_continuations() {
    let (_, opcodes) = TestParser::new(
        r#"
external function generator

function resume {
    continuation.new r2, generator, r0
    continuation.resume r3:r4, r5, r6:r7, r2, r1 => b0 | b1 | b2

b0:
    return r3:r4

b1:
    return r6:r7

b2:
    unwind.resume
}
"#,
    )
    .parse_opcodes(FunctionId(1));

    assert_eq!(
        opcodes,
        vec![
            Opcode::CONTINUATION_NEW,
            Opcode::CONTINUATION_RESUME,
            Opcode::RETURN,
            Opcode::RETURN,
            Opcode::UNWIND_RESUME,
        ]
    );

    let (_, opcodes) = TestParser::new(
        r#"
external function generator

function complete {
    continuation.new r2, generator, r0
    continuation.complete r3:r4, r5, r6:r7, r2, r1 => b0 | b1 | b2

b0:
    continuation.destroy r5
    return r3:r4

b1:
    return r6:r7

b2:
    unwind.resume
}
"#,
    )
    .parse_opcodes(FunctionId(1));

    assert_eq!(
        opcodes,
        vec![
            Opcode::CONTINUATION_NEW,
            Opcode::CONTINUATION_COMPLETE,
            Opcode::CONTINUATION_DESTROY,
            Opcode::RETURN,
            Opcode::RETURN,
            Opcode::UNWIND_RESUME,
        ]
    );
}
