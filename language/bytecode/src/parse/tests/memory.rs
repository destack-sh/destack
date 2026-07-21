use crate::{FrameSlot, FunctionId, MemoryOperation, Opcode, Scalar, TypeId};

use super::TestParser;

/// Parse frame slots and scalar memory operations.
#[test]
fn test_parse_frame_memory() {
    let (object, opcodes) = TestParser::new(
        r#"
type Pair

export function update(r0: int32, r1: address, r2: address, r3: uint64): int32 {
    slot s0: Pair

    r4: address = frame.address s0
    store.int32 r4, r0
    r5: int32 = load.int32 r4
    copyBytes r1 -> r2, r3
    prefetchRead r1
    return r5
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(object.frame_slots(), &[FrameSlot { ty: TypeId(0) }]);
    assert_eq!(
        opcodes,
        vec![
            Opcode::FRAME_ADDRESS,
            Opcode::memory(MemoryOperation::Store, Scalar::Int32),
            Opcode::memory(MemoryOperation::Load, Scalar::Int32),
            Opcode::COPY_BYTES,
            Opcode::PREFETCH_READ,
            Opcode::RETURN,
        ]
    );
}
