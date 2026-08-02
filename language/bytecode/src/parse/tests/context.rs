use crate::{FunctionId, Opcode, RelocationTag};

use super::TestParser;

/// Parse current, replacement, extension, and lookup context operations.
#[test]
fn test_parse_context_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
function scope {
    context.current r3
    context.bind r4, r3, r0, r1:r2, a0, 16
    context.replace r5, r4
    context.get r6:r7, r4, r0, r1:r2, 16
    context.replace r8, r5
    return r6:r7
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::CONTEXT_CURRENT,
            Opcode::CONTEXT_BIND,
            Opcode::CONTEXT_REPLACE,
            Opcode::CONTEXT_GET,
            Opcode::CONTEXT_REPLACE,
            Opcode::RETURN,
        ]
    );
    assert_eq!(
        object
            .relocations()
            .iter()
            .map(|relocation| relocation.tag)
            .collect::<Vec<_>>(),
        vec![RelocationTag::ALLOCATION]
    );
}
