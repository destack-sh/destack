use crate::{
    FunctionId, Initialization, New, NewKind, Opcode, ReferenceKind, Space, Symbol, TypeId,
};

use super::TestParser;

/// Parse value and slice `new` operations with type relocations.
#[test]
fn test_parse_new() {
    let (object, opcodes) = TestParser::new(
        r#"
type Point

export function allocate(r0: uint64): ref<managed, space(local)> {
    r1: ref<managed, space(local)> = new.local.managed.zeroed Point
    r2: uninit<slice<Point, managed, space(local)>> = new.local.managed.slice.uninit Point, r0
    r2: slice<Point, managed, space(local)> = new.complete r2
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
            Opcode::NEW_COMPLETE,
            Opcode::RETURN,
        ]
    );
    assert_eq!(object.instruction_relocations().len(), 2);
    assert!(
        object
            .instruction_relocations()
            .iter()
            .all(|relocation| relocation.symbol == Symbol::ty(TypeId(0).0))
    );
}
