use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse frame and global addresses with every pointer calculation form.
#[test]
fn test_parse_pointer_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    frame.address r4, r2:r3
    global.address.constant r5, g0
    global.address.local r6, g1
    global.address.shared r7, g2
    pointer.add r8, r5, 16
    pointer.add r9, r4, r0
    pointer.add r10, r4, r0, 8
    pointer.diff r11, r10, r8
    return r11
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FRAME_ADDRESS,
            Opcode::GLOBAL_ADDRESS_CONSTANT,
            Opcode::GLOBAL_ADDRESS_LOCAL,
            Opcode::GLOBAL_ADDRESS_SHARED,
            Opcode::POINTER_ADD_IMMEDIATE,
            Opcode::POINTER_ADD,
            Opcode::POINTER_ADD_SCALED,
            Opcode::POINTER_DIFF,
            Opcode::RETURN,
        ]
    );
}
