use crate::{FunctionId, Opcode, VectorOperation};

use super::TestParser;

/// Parse multiword vectors as contiguous register ranges.
#[test]
fn test_parse_vector_registers() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0 {
    vector.add r4:r5, r0:r1, r2:r3: vector<int32, 4>
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
function f0 {
    vector.load r1:r2, r0: vector<int32, 4>
    vector.store r0, r1:r2: vector<int32, 4>
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
