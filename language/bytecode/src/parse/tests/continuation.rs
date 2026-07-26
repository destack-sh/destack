use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse synchronous and receiver-driven continuation ownership.
#[test]
fn test_parse_continuations() {
    let (_, opcodes) = TestParser::new(
        r#"
function* generator(): t0

function owner(): t0 {
    continuation.new r2, generator, r0
    resume r3:r4, r5, r6:r7, r2, r1 => b0 | b1 | b2

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
            Opcode::RESUME,
            Opcode::RETURN,
            Opcode::RETURN,
            Opcode::UNWIND_RESUME,
        ]
    );
}
