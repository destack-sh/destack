use crate::{FunctionId, Opcode, Symbol, TypeId};

use super::TestParser;

/// Parse register value operations and retain their type relocation.
#[test]
fn test_parse_value_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
type User

export function values(r0: int32, r1: boolean, r2: int128): (int32, boolean, typeId, int128) {
    r4: int32 = move r0
    r5: int32 = select r1, r0, r4
    r6: boolean = equal r0, r5
    r7: typeId = type.id User
    r8: int128 = select r1, r2, r2
    return r5, r6, r7, r8
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::MOVE,
            Opcode::SELECT,
            Opcode::EQUAL,
            Opcode::TYPE_ID,
            Opcode::SELECT_RANGE,
            Opcode::RETURN,
        ]
    );
    assert_eq!(object.instruction_relocations().len(), 1);
    assert_eq!(
        object.instruction_relocations()[0].symbol,
        Symbol::ty(TypeId(0).0)
    );
}
