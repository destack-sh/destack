use crate::{
    FrameSlot, FunctionId, GlobalLocation, Opcode, ReferenceKind, ReferenceType, RegisterId, Space,
    TypeId,
};

use super::TestParser;

/// Parse global, frame, reference, offset, index, and distance pointers.
#[test]
fn test_parse_pointer_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
type Pair

local global state: Pair = zero

export function pointers(r0: uint64, r1: ref<managed, space(local)>): int64 {
    slot s0: Pair

    r2: pointer = global.address state
    r3: pointer = frame.address s0
    r4: pointer = reference.pointer r1
    r5: pointer = pointer.offset r2, 16
    r6: pointer = pointer.index r3, r0, stride(8)
    r7: int64 = pointer.distance r6, r5
    return r7
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(object.globals()[0].location, GlobalLocation::LOCAL_STATIC);
    assert_eq!(object.frame_slots(), &[FrameSlot::new(TypeId(0))]);
    assert_eq!(
        opcodes,
        vec![
            Opcode::GLOBAL_ADDRESS,
            Opcode::FRAME_ADDRESS,
            Opcode::REFERENCE_POINTER,
            Opcode::POINTER_OFFSET,
            Opcode::POINTER_INDEX,
            Opcode::POINTER_DISTANCE,
            Opcode::RETURN,
        ]
    );

    // retain the erased reference representation required by direct execution
    let instruction = object
        .instruction(FunctionId(0), 2)
        .expect("valid instruction")
        .expect("reference pointer instruction");
    let mut operands = instruction.operands();
    assert_eq!(operands.register().expect("result register"), RegisterId(4));
    assert_eq!(
        operands.register().expect("reference register"),
        RegisterId(1)
    );
    assert_eq!(
        operands.reference().expect("reference representation"),
        ReferenceType::new(ReferenceKind::MANAGED, Space::LOCAL)
    );
}
