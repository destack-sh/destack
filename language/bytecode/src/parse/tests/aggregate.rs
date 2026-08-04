use crate::{FunctionId, Opcode, Placement, RegisterId, RegisterSpan, RelocationTag};

use super::TestParser;

/// Parse physical aggregate byte placement and variant layout operations.
#[test]
fn test_parse_aggregate_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0 {
    aggregate r2:r3, [r0 @ 0:4, r1 @ 8:8]
    extract r4, r2:r3, 0:4
    insert r5:r6, r2:r3, 8:8, r1
    variant.new r7:r8, l1, 1, r0
    variant.tag r9, r7:r8, l1
    variant.tag.load r10, r0, l1
    variant.tag.load.constant r11, r0, l1
    variant.tag.load.pointer r12, r0, l1
    return r4
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::AGGREGATE,
            Opcode::EXTRACT,
            Opcode::INSERT,
            Opcode::VARIANT_NEW,
            Opcode::VARIANT_TAG,
            Opcode::VARIANT_TAG_LOAD,
            Opcode::VARIANT_TAG_LOAD_CONSTANT,
            Opcode::VARIANT_TAG_LOAD_POINTER,
            Opcode::RETURN,
        ]
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
            RelocationTag::LAYOUT,
            RelocationTag::LAYOUT,
            RelocationTag::LAYOUT,
        ]
    );

    // retain exact source ranges and destination byte spans
    let instruction = object
        .operation(FunctionId(0), 0)
        .expect("valid instruction")
        .expect("aggregate instruction");
    let mut operands = instruction.operands();
    assert_eq!(
        operands.span().expect("result registers"),
        RegisterSpan::new(RegisterId(2), 2)
    );
    assert_eq!(
        operands
            .placements()
            .expect("aggregate placements")
            .collect::<Vec<_>>(),
        vec![
            Placement::new(RegisterSpan::new(RegisterId(0), 1), 0, 4),
            Placement::new(RegisterSpan::new(RegisterId(1), 1), 8, 8),
        ]
    );
}
