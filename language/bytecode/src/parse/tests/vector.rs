use crate::{FunctionId, Opcode, VectorOperation};

use super::TestParser;

/// Parse multiword vectors as contiguous register ranges.
#[test]
fn test_parse_vector_registers() {
    let (object, opcodes) = TestParser::new(
        r#"
export function add(
    r0: vector<int32, 4>,
    r2: vector<int32, 4>,
): vector<int32, 4> {
    r4: vector<int32, 4> = int.add r0, r2
    return r4
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
export function copy(r0: pointer): void {
    r1: vector<int32, 4> = load r0
    store r0, r1
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
