use crate::{FunctionId, Opcode, Symbol, TensorOperation};

use super::TestParser;

/// Parse tensor operations with explicit runtime type relocations.
#[test]
fn test_parse_tensor_operation() {
    let (object, opcodes) = TestParser::new(
        r#"
type Matrix

export function add(
    r0: tensor<int32, Matrix, space(local)>,
    r1: tensor<int32, Matrix, space(local)>,
): tensor<int32, Matrix, space(local)> {
    r2: tensor<int32, Matrix, space(local)> = int.add r0, r1
    return r2
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(
        opcodes,
        vec![Opcode::tensor(TensorOperation::Element), Opcode::RETURN]
    );
    assert_eq!(object.instruction_relocations().len(), 3);
    assert!(
        object
            .instruction_relocations()
            .iter()
            .all(|relocation| relocation.symbol == Symbol::ty(0))
    );
}
