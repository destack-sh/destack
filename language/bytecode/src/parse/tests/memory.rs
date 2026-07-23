use crate::{
    FrameSlot, FunctionId, MemoryOperation, Opcode, RegisterId, RegisterRange, Scalar, TypeId,
};

use super::TestParser;

/// Parse frame slots and scalar memory operations.
#[test]
fn test_parse_frame_memory() {
    let (object, opcodes) = TestParser::new(
        r#"
type Pair

export function update(r0: int32, r1: pointer, r2: pointer, r3: uint64, r4: words<2>): int32 {
    slot s0: Pair = r4[2]

    frame.store s0, r4
    r6: words<2> = frame.load s0
    r8: pointer = frame.address s0
    store.int32 r8, r0
    r9: int32 = load.int32 r8
    copy.bytes r1 -> r2, r3
    prefetch.read r1
    return r9
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(
        object.frame_slots(),
        &[FrameSlot::from_registers(
            TypeId(0),
            RegisterRange::new(RegisterId(4), 2)
        )]
    );
    assert_eq!(
        opcodes,
        vec![
            Opcode::FRAME_STORE,
            Opcode::FRAME_LOAD,
            Opcode::FRAME_ADDRESS,
            Opcode::memory(MemoryOperation::Store, Scalar::Int32),
            Opcode::memory(MemoryOperation::Load, Scalar::Int32),
            Opcode::COPY_BYTES,
            Opcode::PREFETCH_READ,
            Opcode::RETURN,
        ]
    );
}
