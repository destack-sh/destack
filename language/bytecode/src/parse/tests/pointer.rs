use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse every pointer materialization and calculation form.
#[test]
fn test_parse_pointer_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {    frame.address r4, r2:r3
    pointer.frame r5, r4
    global.address r6, g0
    pointer.global r7, r6
    pointer.local r8, r0
    pointer.shared r9, r1
    pointer.add r10, r7, 16
    pointer.add r11, r5, r0
    pointer.add r12, r5, r0, 8
    pointer.byteOffsetFrom r13, r12, r10
    return r13
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FRAME_ADDRESS,
            Opcode::POINTER_FRAME,
            Opcode::GLOBAL_ADDRESS,
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
