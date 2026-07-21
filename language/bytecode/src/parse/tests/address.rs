use crate::{FrameSlot, FunctionId, GlobalLocation, Opcode, TypeId};

use super::TestParser;

/// Parse global, frame, offset, element, and distance addresses.
#[test]
fn test_parse_address_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
type Pair

local global state: Pair = zero

export function addresses(r0: uint64): int64 {
    slot s0: Pair

    r1: address = global.address state
    r2: address = frame.address s0
    r3: address = address.offset r1, 16
    r4: address = address.element r2, r0, stride(8)
    r5: int64 = address.distance r4, r3
    return r5
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(object.globals()[0].location, GlobalLocation::LOCAL_STATIC);
    assert_eq!(object.frame_slots(), &[FrameSlot { ty: TypeId(0) }]);
    assert_eq!(
        opcodes,
        vec![
            Opcode::GLOBAL_ADDRESS,
            Opcode::FRAME_ADDRESS,
            Opcode::ADDRESS_OFFSET,
            Opcode::ADDRESS_ELEMENT,
            Opcode::ADDRESS_DISTANCE,
            Opcode::RETURN,
        ]
    );
}
