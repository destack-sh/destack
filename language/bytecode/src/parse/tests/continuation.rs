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
    continuation.resume r3:r4, r2, r1
    return r3:r4
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
        ]
    );
}
