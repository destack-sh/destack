use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse debugger and profiling instructions with function-local counters.
#[test]
fn test_parse_runtime_instructions() {
    let (object, opcodes) = TestParser::new(
        r#"
export function observed(r0: uint64): void {
    breakpoint
    safepoint
    profile.increment counter(4)
    profile.sample sampler(7), r0
    return
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(
        opcodes,
        vec![
            Opcode::BREAKPOINT,
            Opcode::SAFEPOINT,
            Opcode::PROFILE_INCREMENT,
            Opcode::PROFILE_SAMPLE,
            Opcode::RETURN,
        ]
    );
    assert_eq!(object.functions()[0].counter_count, 5);
    assert_eq!(object.functions()[0].sampler_count, 8);
}
