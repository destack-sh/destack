use crate::{
    FunctionId, Opcode, ReferenceKind, ReferenceType, RegisterId, RegisterRange, Space, TypeId,
    ValueType,
};

use super::TestParser;

/// Parse reference loads, stores, lifetime operations, barriers, drops, and releases.
#[test]
fn test_parse_reference_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
type Point

export function references(
    r0: pointer,
    r1: ref<managed, space(local)>,
    r2: ref<unique, space(local)>,
    r3: uint64,
): ref<managed, space(local)> {
    r4: ref<managed, space(local)> = load r0, Point
    store r0, r4, Point
    pin r4
    unpin r4
    barrier r4, r3, r3
    drop r0: Point
    free r2
    return r4
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::LOAD,
            Opcode::STORE,
            Opcode::PIN,
            Opcode::UNPIN,
            Opcode::BARRIER,
            Opcode::DROP,
            Opcode::FREE,
            Opcode::RETURN,
        ]
    );
    assert_eq!(object.instruction_relocations().len(), 3);

    // retain the managed reference representation for collector operations
    let instruction = object
        .instruction(FunctionId(0), 4)
        .expect("valid instruction")
        .expect("barrier instruction");
    let mut operands = instruction.operands();
    assert_eq!(operands.register().expect("object register"), RegisterId(4));
    assert_eq!(
        operands.reference().expect("object representation"),
        ReferenceType::new(ReferenceKind::MANAGED, Space::LOCAL)
    );

    // retain the complete logical value and concrete destructor type
    let instruction = object
        .instruction(FunctionId(0), 5)
        .expect("valid instruction")
        .expect("drop instruction");
    let mut operands = instruction.operands();
    assert_eq!(
        operands.range().expect("value registers"),
        RegisterRange::new(RegisterId(0), ValueType::pointer().word_count())
    );
    assert_eq!(operands.u32().expect("dropped type"), TypeId(0).0);

    // retain the unique local representation required to free the allocation
    let instruction = object
        .instruction(FunctionId(0), 6)
        .expect("valid instruction")
        .expect("free instruction");
    let mut operands = instruction.operands();
    assert_eq!(
        operands.register().expect("reference register"),
        RegisterId(2)
    );
    assert_eq!(
        operands.reference().expect("reference representation"),
        ReferenceType::new(ReferenceKind::UNIQUE, Space::LOCAL)
    );
}
