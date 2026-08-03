use crate::{FunctionId, MemoryOperation, Opcode, Scalar};

use super::TestParser;

/// Parse scalar, packed, and ranged memory operations into exact opcodes.
#[test]
fn test_parse_memory() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    frame.address r5, r4
    store r5, r0: int32
    load r7, r5: int32
    store r5, r8:r9, 12
    load r8:r9, r5, 12
    memory.copy r2, r1, r3
    memory.move r2, r1, 16
    memory.fill r2, r0, r3
    memory.compare r10, r2, r1, 16
    prefetch.read r1
    return r7
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FRAME_ADDRESS,
            Opcode::memory(MemoryOperation::Store, Scalar::Int32),
            Opcode::memory(MemoryOperation::Load, Scalar::Int32),
            Opcode::STORE,
            Opcode::LOAD,
            Opcode::MEMORY_COPY,
            Opcode::MEMORY_MOVE_IMMEDIATE,
            Opcode::MEMORY_FILL,
            Opcode::MEMORY_COMPARE_IMMEDIATE,
            Opcode::PREFETCH_READ,
            Opcode::RETURN,
        ]
    );
}
