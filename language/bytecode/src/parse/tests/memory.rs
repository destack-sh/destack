use crate::{Address, FunctionId, MemoryOperation, Opcode, Prefetch, Scalar, Transfer};

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
    load.constant r6, r4: uint64
    store.pointer r11, r1: uint16
    store r5, r8:r9, 12
    load.pointer r8:r9, r11, 12
    memory.copy r2, r1, r3
    memory.move.pointer.memory r2, r1, 16
    memory.fill.pointer r2, r0, r3
    memory.compare.constant.pointer r10, r2, r1, 16
    prefetch.read.pointer r1
    return r7
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FRAME_ADDRESS,
            Opcode::memory(MemoryOperation::Store, Address::Memory, Scalar::Int32)
                .expect("memory store opcode"),
            Opcode::memory(MemoryOperation::Load, Address::Memory, Scalar::Int32)
                .expect("memory load opcode"),
            Opcode::memory(MemoryOperation::Load, Address::Constant, Scalar::Uint64)
                .expect("constant load opcode"),
            Opcode::memory(MemoryOperation::Store, Address::Pointer, Scalar::Uint16)
                .expect("pointer store opcode"),
            Opcode::STORE,
            Opcode::LOAD_POINTER,
            Opcode::transfer(Transfer::Copy, Address::Memory, Address::Memory, false)
                .expect("memory copy opcode"),
            Opcode::transfer(Transfer::Move, Address::Pointer, Address::Memory, true)
                .expect("pointer move opcode"),
            Opcode::fill(Address::Pointer, false).expect("pointer fill opcode"),
            Opcode::compare(Address::Constant, Address::Pointer, true),
            Opcode::prefetch(Prefetch::Read, Address::Pointer).expect("pointer prefetch opcode"),
            Opcode::RETURN,
        ]
    );
}
