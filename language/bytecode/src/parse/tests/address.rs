use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse stable addresses and both arithmetic representations.
#[test]
fn test_parse_address_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    frame.address r4, r2:r3
    global.address.constant r5, g0
    global.address.local r6, g1
    global.address.shared r7, g2
    reference.add r8, r5, 16
    reference.add r9, r4, r0
    reference.add r10, r4, r0, 8
    reference.diff r11, r10, r8
    pointer.add r12, r1, 16
    pointer.diff r13, r12, r1
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
            Opcode::REFERENCE_ADD_IMMEDIATE,
            Opcode::REFERENCE_ADD,
            Opcode::REFERENCE_ADD_SCALED,
            Opcode::REFERENCE_DIFF,
            Opcode::POINTER_ADD_IMMEDIATE,
            Opcode::POINTER_DIFF,
            Opcode::RETURN,
        ]
    );
}
