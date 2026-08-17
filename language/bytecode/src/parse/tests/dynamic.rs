use crate::{FunctionId, Opcode, RelocationTag};

use super::TestParser;

/// Parse dynamic binding, payload, field, and runtime type projections.
#[test]
fn test_parse_dynamic_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0 {
    dynamic.bind r1:r2, r0, d0
    move r3, r1
    dynamic.read r3, r1:r2[1], 8
    dynamic.type r4, r1:r2
    return r1:r4
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::DYNAMIC_BIND,
            Opcode::MOVE,
            Opcode::DYNAMIC_READ,
            Opcode::DYNAMIC_TYPE,
            Opcode::RETURN
        ]
    );
    let [relocation] = object.relocations() else {
        panic!("dynamic binding should produce one relocation");
    };
    assert_eq!(relocation.tag, RelocationTag::DYNAMIC);
}
