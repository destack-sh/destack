use crate::{AtomicOperation, FunctionId, Opcode, Scalar};

use super::TestParser;

/// Parse atomic operations into scalar-specialized opcodes.
#[test]
fn test_parse_atomic_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    atomic.load.uint32 r2, r0, acquire
    atomic.store.uint32 r0, r1, release
    atomic.rmw.add.uint32 r2, r0, r1, acquireRelease
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    return r2
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::atomic(AtomicOperation::Load, Scalar::Uint32).expect("atomic opcode"),
            Opcode::atomic(AtomicOperation::Store, Scalar::Uint32).expect("atomic opcode"),
            Opcode::atomic(AtomicOperation::FetchAdd, Scalar::Uint32).expect("atomic opcode"),
            Opcode::ATOMIC_FENCE,
            Opcode::RETURN,
        ]
    );
}
