use crate::{FunctionId, MemoryOperation, Opcode, Scalar};

use super::TestParser;

/// Parse register addresses and scalar memory operations.
#[test]
fn test_parse_memory() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    address r6, r4
    store.int32 r6, r0
    load.int32 r7, r6
    copy.bytes r1 -> r2, r3
    prefetch.read r1
    return r7
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::ADDRESS,
            Opcode::memory(MemoryOperation::Store, Scalar::Int32),
            Opcode::memory(MemoryOperation::Load, Scalar::Int32),
            Opcode::COPY_BYTES,
            Opcode::PREFETCH_READ,
            Opcode::RETURN,
        ]
    );
}
