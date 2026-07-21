use crate::{FunctionId, Opcode};

use super::TestParser;

/// Parse reference loads, stores, lifetime operations, barriers, drops, and releases.
#[test]
fn test_parse_reference_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
type Point

export function references(
    r0: address,
    r1: ref<managed, space(local)>,
    r2: ref<unique, space(local)>,
    r3: uint64,
): ref<managed, space(local)> {
    r4: ref<managed, space(local)> = load r0
    store r0, r4
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
}
