use crate::{
    FunctionId, Opcode, ReferenceKind, ReferenceType, RegisterId, RegisterSpan, RelocationTag,
    Space, ValueType,
};

use super::TestParser;

/// Parse reference loads, stores, lifetime operations, barriers, drops, and releases.
#[test]
fn test_parse_reference_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0(): t0
function f1(): t0 {    load r4, r0, 8
    store r0, r4, 8
    pin.local.managed r4
    unpin.local.managed r4
    barrier.local.managed r4, r3, r3
    drop r0, f0
    free.local.unique r2
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
        ReferenceType::new(ReferenceKind::MANAGED, Space::LOCAL)
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

    // retain the unique local representation required to free the allocation
    let instruction = object
        .operation(FunctionId(1), 6)
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
