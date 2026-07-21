use crate::{FunctionId, Opcode, Symbol, TensorOperation};

use super::TestParser;

/// Parse tensor operations with explicit runtime type relocations.
#[test]
fn test_parse_tensor_operation() {
    let (object, opcodes) = TestParser::new(
        r#"
type Matrix

export function add(
    r0: tensor<int32, Matrix>,
    r1: tensor<int32, Matrix>,
): tensor<int32, Matrix> {
    r2: tensor<int32, Matrix> = int.add r0, r1
    return r2
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(
        opcodes,
        vec![Opcode::tensor(TensorOperation::Element), Opcode::RETURN]
    );
    assert_eq!(object.instruction_relocations().len(), 1);
    assert_eq!(object.instruction_relocations()[0].symbol, Symbol::ty(0));
}
