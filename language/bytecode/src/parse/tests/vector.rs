use crate::{FunctionId, Opcode, VectorOperation};

use super::TestParser;

/// Parse multiword vectors as contiguous register ranges.
#[test]
fn test_parse_vector_registers() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    vector.add.int32x4 r4:r5, r0:r1, r2:r3
    return r4:r5
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![Opcode::vector(VectorOperation::Element), Opcode::RETURN]
    );
    assert_eq!(object.functions()[0].register_count, 6);
}

/// Parse vector memory operations into their exact opcodes.
#[test]
fn test_parse_vector_memory() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    vector.load.int32x4 r1:r2, r0
    vector.store.int32x4 r0, r1:r2
    return
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::vector(VectorOperation::Load),
            Opcode::vector(VectorOperation::Store),
            Opcode::RETURN,
        ]
    );
}
