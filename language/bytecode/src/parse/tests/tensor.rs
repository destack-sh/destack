use crate::{FunctionId, Opcode, RelocationTag, TensorOperation};

use super::TestParser;

/// Parse tensor operations with direct layout and allocation relocations.
#[test]
fn test_parse_tensor_operation() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    tensor.element r2, [(r0, l0), (r1, l0)], int.add, a0
    return r2
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(
        opcodes,
        vec![Opcode::tensor(TensorOperation::Element), Opcode::RETURN]
    );
    assert_eq!(
        object
            .relocations()
            .iter()
            .map(|relocation| relocation.tag)
            .collect::<Vec<_>>(),
        vec![
            RelocationTag::LAYOUT,
            RelocationTag::LAYOUT,
            RelocationTag::ALLOCATION,
        ]
    );
}
