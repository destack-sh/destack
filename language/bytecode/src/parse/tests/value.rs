use crate::{FunctionId, Opcode, RelocationTag};

use super::TestParser;

/// Parse register value operations and retain their type relocation.
#[test]
fn test_parse_value_operations() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    move r4, r0
    move r0, r1
    select r5, r1, r4, r4
    equal r6, r4, r5
    constant.type r7, t0
    select r8:r9, r1, r2:r3, r2:r3
    constant.null r10
    constant.undefined r11
    return r5:r11
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::MOVE,
            Opcode::MOVE,
            Opcode::SELECT,
            Opcode::EQUAL,
            Opcode::CONSTANT_TYPE,
            Opcode::SELECT_RANGE,
            Opcode::CONSTANT_NULL,
            Opcode::CONSTANT_UNDEFINED,
            Opcode::RETURN,
        ]
    );
    assert_eq!(object.relocations().len(), 1);
    assert_eq!(object.relocations()[0].tag, RelocationTag::TYPE);
}

/// Parse explicit zero initialization for opaque storage.
#[test]
fn test_parse_storage_values() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    constant.zeroed r0
    return r0
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(opcodes, vec![Opcode::CONSTANT_ZEROED, Opcode::RETURN]);
}
