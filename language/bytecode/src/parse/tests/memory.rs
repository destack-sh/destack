use crate::{Address, FunctionId, MemoryOperation, Opcode, Prefetch, Scalar, Transfer};

use super::TestParser;

/// Parse scalar, packed, and ranged memory operations into exact opcodes.
#[test]
fn test_parse_memory() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    frame.address r5, r4
    store.int32 r5, r0
    load.int32 r7, r5
    load.uint64 r6, r4
    store.uint16 pointer r11, r1
    load.volatile.uint32 r10, r5
    store.volatile.uint32 pointer r11, r10
    memory.store r5, r8:r9, 12
    memory.load r8:r9, pointer r11, 12
    memory.store.volatile r5, r8:r9, 12
    memory.load.volatile r8:r9, pointer r11, 12
    memory.copy r2, r1, r3
    memory.move pointer r2, r1, 16
    memory.fill pointer r2, r0, r3
    memory.compare r10, r2, pointer r1, 16
    prefetch.read pointer r1
    return r7
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FRAME_ADDRESS,
            Opcode::memory(
                MemoryOperation::Store,
                Address::Reference,
                Scalar::Int32,
                false,
            ),
            Opcode::memory(
                MemoryOperation::Load,
                Address::Reference,
                Scalar::Int32,
                false,
            ),
            Opcode::memory(
                MemoryOperation::Load,
                Address::Reference,
                Scalar::Uint64,
                false,
            ),
            Opcode::memory(
                MemoryOperation::Store,
                Address::Pointer,
                Scalar::Uint16,
                false,
            ),
            Opcode::memory(
                MemoryOperation::Load,
                Address::Reference,
                Scalar::Uint32,
                true,
            ),
            Opcode::memory(
                MemoryOperation::Store,
                Address::Pointer,
                Scalar::Uint32,
                true,
            ),
            Opcode::STORE,
            Opcode::LOAD_POINTER,
            Opcode::STORE_VOLATILE,
            Opcode::LOAD_VOLATILE_POINTER,
            Opcode::transfer(
                Transfer::Copy,
                Address::Reference,
                Address::Reference,
                false,
            ),
            Opcode::transfer(Transfer::Move, Address::Pointer, Address::Reference, true,),
            Opcode::fill(Address::Pointer, false),
            Opcode::compare(Address::Reference, Address::Pointer, true),
            Opcode::prefetch(Prefetch::Read, Address::Pointer),
            Opcode::RETURN,
        ]
    );
}
