use crate::{FunctionId, Opcode, Symbol};

use super::TestParser;

/// Parse closure binding, environment access, and indirect calls.
#[test]
fn test_parse_function_values() {
    let (object, opcodes) = TestParser::new(
        r#"
function body(environment r0: ref<managed, space(local)>, r1: int32): int32 {
    r2: ref<managed, space(local)> = function.environment.current
    return r1
}

export function apply(r0: ref<managed, space(local)>, r1: int32): int32 {
    r2: function = function.bind body, r0
    r4: ref<managed, space(local)> = function.environment r2
    r5: int32 = call.indirect r2(r1)
    return r5
}
"#,
    )
    .parse_opcodes(FunctionId(1));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FUNCTION_BIND,
            Opcode::FUNCTION_ENVIRONMENT,
            Opcode::CALL_INDIRECT,
            Opcode::RETURN,
        ]
    );
    assert_eq!(
        object
            .instruction_relocations()
            .iter()
            .map(|relocation| relocation.symbol)
            .collect::<Vec<_>>(),
        vec![Symbol::function(0)]
    );
}
