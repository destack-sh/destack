use crate::{Address, AtomicOperation, FunctionId, Opcode, Scalar};

use super::TestParser;

/// Parse atomic operations into scalar-specialized opcodes.
#[test]
fn test_parse_atomic_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    atomic.load r2, r0, acquire: uint32
    atomic.store.pointer r0, r1, release: uint32
    atomic.rmw.add r2, r0, r1, acquireRelease: uint32
    atomic.cas r2, r3, r0, r1, r4, acquireRelease, failure(acquire): uint32
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    return r2
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::atomic(AtomicOperation::Load, Address::Memory, Scalar::Uint32)
                .expect("atomic opcode"),
            Opcode::atomic(AtomicOperation::Store, Address::Pointer, Scalar::Uint32)
                .expect("atomic opcode"),
            Opcode::atomic(AtomicOperation::FetchAdd, Address::Memory, Scalar::Uint32)
                .expect("atomic opcode"),
            Opcode::atomic(
                AtomicOperation::CompareExchange,
                Address::Memory,
                Scalar::Uint32,
            )
            .expect("atomic opcode"),
            Opcode::ATOMIC_FENCE,
            Opcode::RETURN,
        ]
    );
}
