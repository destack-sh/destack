use crate::{FunctionId, Opcode, TypeId};

use super::TestParser;

/// Parse dynamic binding and its payload and type projections.
#[test]
fn test_parse_dynamic_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
type Constraint
type Concrete

export function dynamicValue(r0: ref<managed, space(shared)>): (
    dynamic<Constraint, space(shared)>,
    ref<managed, space(shared)>,
    typeId
) {
    r1: dynamic<Constraint, space(shared)> = dynamic.bind r0: Concrete
    r3: ref<managed, space(shared)> = dynamic.payload r1
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
    let [relocation] = object.dynamic_relocations() else {
        panic!("dynamic binding should produce one relocation");
    };
    assert_eq!(relocation.concrete, TypeId(1));
    assert_eq!(relocation.constraint, TypeId(0));
}
