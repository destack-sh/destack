use crate::{FunctionId, Opcode, RelocationTag};

use super::TestParser;

/// Parse closure binding, physical environment extraction, and indirect calls.
#[test]
fn test_parse_function_values() {
    let (object, opcodes) = TestParser::new(
        r#"
external function f0

function f1 {    function.bind r2:r3, f0, r0
    extract r4, r2:r3, 8, 8
    call.indirect r5, r2:r3, r1
    return r5
}
"#,
    )
    .parse_opcodes(FunctionId(1));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FUNCTION_BIND,
            Opcode::EXTRACT,
            Opcode::CALL_INDIRECT,
            Opcode::RETURN,
        ]
    );
    assert_eq!(
        object
            .relocations()
            .iter()
            .map(|relocation| relocation.tag)
            .collect::<Vec<_>>(),
        vec![RelocationTag::FUNCTION]
    );
}
