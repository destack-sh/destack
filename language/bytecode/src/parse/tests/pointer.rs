use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse every pointer materialization and calculation form.
#[test]
fn test_parse_pointer_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {    pointer.frame r4, r2:r3
    pointer.global r5, g0
    pointer.local r6, r0
    pointer.shared r7, r1
    pointer.add r8, r5, 16
    pointer.add r9, r4, r0
    pointer.add r10, r4, r0, 8
    pointer.byteOffsetFrom r11, r10, r8
    return r11
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::POINTER_FRAME,
            Opcode::POINTER_GLOBAL,
            Opcode::POINTER_LOCAL,
            Opcode::POINTER_SHARED,
            Opcode::POINTER_ADD_IMMEDIATE,
            Opcode::POINTER_ADD,
            Opcode::POINTER_ADD_SCALED,
            Opcode::POINTER_BYTE_OFFSET_FROM,
            Opcode::RETURN,
        ]
    );
}
