use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse stable addresses and both arithmetic representations.
#[test]
fn test_parse_address_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    frame.address r4, r2:r3
    global.address r5, g0
    global.address r6, g1
    global.address r7, g2
    address.add r8, r5, 16
    address.add r9, r4, r0
    address.add r10, r4, r0, 8
    address.diff r11, r10, r8
    address.add r12, r1, 16
    address.diff r13, r12, r1
    return r11
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FRAME_ADDRESS,
            Opcode::GLOBAL_ADDRESS,
            Opcode::GLOBAL_ADDRESS,
            Opcode::GLOBAL_ADDRESS,
            Opcode::ADDRESS_ADD_IMMEDIATE,
            Opcode::ADDRESS_ADD,
            Opcode::ADDRESS_ADD_SCALED,
            Opcode::ADDRESS_DIFF,
            Opcode::ADDRESS_ADD_IMMEDIATE,
            Opcode::ADDRESS_DIFF,
            Opcode::RETURN,
        ]
    );
}

/// Parse rebases between world references and native pointers.
#[test]
fn test_parse_address_rebases() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    address.pointer r1, r0
    address.reference r2, r1
    return r2
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::ADDRESS_POINTER,
            Opcode::ADDRESS_REFERENCE,
            Opcode::RETURN,
        ]
    );
}
