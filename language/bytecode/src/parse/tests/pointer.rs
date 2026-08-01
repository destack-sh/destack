use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse every pointer materialization and calculation form.
#[test]
fn test_parse_pointer_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {    frame.address r4, r2:r3
    pointer.frame r5, r4
    global.address.constant r6, g0
    global.address.local r7, g1
    global.address.shared r8, g2
    pointer.constant r9, r6
    pointer.memory r10, r7
    pointer.add r11, r9, 16
    pointer.add r12, r5, r0
    pointer.add r13, r5, r0, 8
    pointer.byteOffsetFrom r14, r13, r11
    return r14
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FRAME_ADDRESS,
            Opcode::POINTER_FRAME,
            Opcode::GLOBAL_ADDRESS_CONSTANT,
            Opcode::GLOBAL_ADDRESS_LOCAL,
            Opcode::GLOBAL_ADDRESS_SHARED,
            Opcode::POINTER_CONSTANT,
            Opcode::POINTER_MEMORY,
            Opcode::POINTER_ADD_IMMEDIATE,
            Opcode::POINTER_ADD,
            Opcode::POINTER_ADD_SCALED,
            Opcode::POINTER_BYTE_OFFSET_FROM,
            Opcode::RETURN,
        ]
    );
}
