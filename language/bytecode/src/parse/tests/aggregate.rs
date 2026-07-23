use crate::{FunctionId, Opcode, RegisterId, RegisterRange, Symbol, TypeId};

use super::TestParser;

/// Parse packed aggregate and variant operations with their runtime type relocations.
#[test]
fn test_parse_aggregate_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
type Pair
type Choice

export function values(r0: int32, r1: int32, r2: words<2>): int32 {
    r4: words<2> = aggregate Pair (r0, r1)
    r6: int32 = field.get r4, Pair, 0
    r7: words<2> = field.set r4, Pair, 1, r1
    r9: int32 = element.get r7, Pair, 0
    r10: words<2> = element.set r7, Pair, 1, r0
    r12: words<2> = variant.new Choice, 1, r1
    r14: uint32 = variant.tag r12, Choice
    r15: int32 = variant.payload r12, Choice, 1
    return r15
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::AGGREGATE,
            Opcode::FIELD_GET,
            Opcode::FIELD_SET,
            Opcode::ELEMENT_GET,
            Opcode::ELEMENT_SET,
            Opcode::VARIANT_NEW,
            Opcode::VARIANT_TAG,
            Opcode::VARIANT_PAYLOAD,
            Opcode::RETURN,
        ]
    );
    assert_eq!(object.instruction_relocations().len(), 8);
    assert_eq!(
        object.instruction_relocations()[0].symbol,
        Symbol::ty(TypeId(0).0)
    );
    assert_eq!(
        object.instruction_relocations()[5].symbol,
        Symbol::ty(TypeId(1).0)
    );

    // retain packed source and replacement ranges for persistent field updates
    let instruction = object
        .instruction(FunctionId(0), 2)
        .expect("valid instruction")
        .expect("field update instruction");
    let mut operands = instruction.operands();
    assert_eq!(
        operands.range().expect("result registers"),
        RegisterRange::new(RegisterId(7), 2)
    );
    assert_eq!(
        operands.range().expect("source registers"),
        RegisterRange::new(RegisterId(4), 2)
    );
    assert_eq!(operands.u32().expect("aggregate type"), TypeId(0).0);
    assert_eq!(operands.u32().expect("field index"), 1);
    assert_eq!(
        operands.range().expect("replacement registers"),
        RegisterRange::new(RegisterId(1), 1)
    );
}
