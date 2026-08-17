use crate::{
    FunctionId, Opcode, ReferenceKind, ReferenceType, RegisterId, RegisterSpan, RelocationTag,
    Storage, ValueType,
};

use super::TestParser;

/// Parse reference loads, stores, lifetime operations, barriers, drops, and releases.
#[test]
fn test_parse_reference_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
external function f0
function f1 {
    memory.load r4, r0, 8
    memory.store r0, r4, 8
    pin r4: ref<managed, local>
    unpin r4: ref<managed, local>
    barrier r4, r3, r3: ref<managed, local>
    drop r0, f0
    drop r2
    free r2
    return r4
}
"#,
    )
    .parse_opcodes(FunctionId(1));

    assert_eq!(
        opcodes,
        vec![
            Opcode::LOAD,
            Opcode::STORE,
            Opcode::PIN,
            Opcode::UNPIN,
            Opcode::BARRIER,
            Opcode::DROP,
            Opcode::DROP_INDIRECT,
            Opcode::FREE,
            Opcode::RETURN,
        ]
    );
    assert_eq!(
        object
            .relocations()
            .iter()
            .map(|relocation| relocation.tag)
            .collect::<Vec<_>>(),
        vec![RelocationTag::FUNCTION]
    );

    // retain the managed reference representation for collector operations
    let instruction = object
        .operation(FunctionId(1), 4)
        .expect("valid instruction")
        .expect("barrier instruction");
    let mut operands = instruction.operands();
    assert_eq!(operands.register().expect("object register"), RegisterId(4));
    assert_eq!(
        operands.reference().expect("object representation"),
        ReferenceType::new(ReferenceKind::MANAGED, Storage::LOCAL)
    );

    // retain the complete logical value and direct destructor identity
    let instruction = object
        .operation(FunctionId(1), 5)
        .expect("valid instruction")
        .expect("drop instruction");
    let mut operands = instruction.operands();
    assert_eq!(
        operands.span().expect("value registers"),
        RegisterSpan::new(RegisterId(0), ValueType::pointer().word_count())
    );
    assert_eq!(operands.u32().expect("destructor"), 0);

    // retain the erased allocation owner selected for indirect destruction
    let instruction = object
        .operation(FunctionId(1), 6)
        .expect("valid instruction")
        .expect("indirect drop instruction");
    let mut operands = instruction.operands();
    assert_eq!(operands.register().expect("owner register"), RegisterId(2));

    // retain the allocation owner without duplicating its heap space
    let instruction = object
        .operation(FunctionId(1), 7)
        .expect("valid instruction")
        .expect("free instruction");
    let mut operands = instruction.operands();
    assert_eq!(operands.register().expect("owner register"), RegisterId(2));
}
