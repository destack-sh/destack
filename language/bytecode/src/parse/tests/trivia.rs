use crate::{FunctionId, Opcode};

use super::TestParser;

/// Ignore comments, blank lines, and mixed line endings between bytecode tokens.
#[test]
fn test_parse_bytecode_trivia() {
    let (_, opcodes) = TestParser::new(
        r#"// declaration

function f0 {
// stop
breakpoint
return
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(opcodes, vec![Opcode::BREAKPOINT, Opcode::RETURN]);
}
