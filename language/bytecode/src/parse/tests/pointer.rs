use crate::{FunctionId, Opcode, ReferenceKind, ReferenceType, RegisterId, Space};

use super::TestParser;

/// Parse global, register, reference, additive, and distance pointer operations.
#[test]
fn test_parse_pointer_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    global.address r4, g0
    address r5, r2:r3
    reference.pointer.local.managed r6, r1
    pointer.add r7, r4, 16
    pointer.add r8, r5, r0
    pointer.add r9, r5, r0, 8
    pointer.distance r10, r9, r7
    return r10
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::GLOBAL_ADDRESS,
            Opcode::ADDRESS,
            Opcode::REFERENCE_POINTER,
            Opcode::POINTER_ADD_IMMEDIATE,
            Opcode::POINTER_ADD,
            Opcode::POINTER_ADD_SCALED,
            Opcode::POINTER_DISTANCE,
            Opcode::RETURN,
        ]
    );

    // retain the reference representation required to resolve its pointer
    let instruction = object
        .operation(FunctionId(0), 2)
        .expect("valid instruction")
        .expect("reference pointer instruction");
    let mut operands = instruction.operands();
    assert_eq!(operands.register().expect("result register"), RegisterId(6));
    assert_eq!(
        operands.register().expect("reference register"),
        RegisterId(1)
    );
    assert_eq!(
        operands.reference().expect("reference representation"),
        ReferenceType::new(ReferenceKind::MANAGED, Space::LOCAL)
    );
}
