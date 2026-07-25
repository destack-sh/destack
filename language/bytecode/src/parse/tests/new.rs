use crate::{
    FunctionId, Initialization, New, NewKind, Opcode, ReferenceKind, RelocationTag, Space,
};

use super::TestParser;

/// Parse value and slice allocations with direct allocation site relocations.
#[test]
fn test_parse_new() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    new.local.managed.zeroed r1, a0
    new.local.managed.slice.uninit r2:r3, a1, r0
    return r1
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(
        opcodes,
        vec![
            Opcode::new(New {
                space: Space::LOCAL,
                ownership: ReferenceKind::MANAGED,
                kind: NewKind::Value,
                initialization: Initialization::Zeroed,
                is_fallible: false,
            })
            .expect("new opcode"),
            Opcode::new(New {
                space: Space::LOCAL,
                ownership: ReferenceKind::MANAGED,
                kind: NewKind::Slice,
                initialization: Initialization::Uninit,
                is_fallible: false,
            })
            .expect("new opcode"),
            Opcode::RETURN,
        ]
    );
    assert_eq!(
        object
            .relocations()
            .iter()
            .map(|relocation| relocation.tag)
            .collect::<Vec<_>>(),
        vec![RelocationTag::ALLOCATION, RelocationTag::ALLOCATION]
    );
}
