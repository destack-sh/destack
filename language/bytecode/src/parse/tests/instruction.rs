use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse debugger and profiling instructions with function-local counters.
#[test]
fn test_parse_runtime_instructions() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    breakpoint
    profile.increment c4
    profile.sample s7, r0
    return
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(
        opcodes,
        vec![
            Opcode::BREAKPOINT,
            Opcode::PROFILE_INCREMENT,
            Opcode::PROFILE_SAMPLE,
            Opcode::RETURN,
        ]
    );
}
