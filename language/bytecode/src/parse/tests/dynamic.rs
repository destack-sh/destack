use crate::{FunctionId, Opcode, Symbol};

use super::TestParser;

/// Parse dynamic binding and its payload and type projections.
#[test]
fn test_parse_dynamic_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
type Constraint
type Concrete

export function dynamicValue(r0: ref<managed, space(local)>): (
    dynamic<Constraint>,
    ref<managed, space(local)>,
    typeId
) {
    r1: dynamic<Constraint> = dynamic.bind r0: Concrete
    r3: ref<managed, space(local)> = dynamic.payload r1
    r4: typeId = dynamic.type r1
    return r1, r3, r4
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::DYNAMIC_BIND,
            Opcode::DYNAMIC_PAYLOAD,
            Opcode::DYNAMIC_TYPE,
            Opcode::RETURN
        ]
    );
    assert_eq!(
        object
            .instruction_relocations()
            .iter()
            .map(|relocation| relocation.symbol)
            .collect::<Vec<_>>(),
        vec![Symbol::ty(1), Symbol::ty(0)]
    );
}
