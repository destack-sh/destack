use crate::{FunctionId, Linkage, Opcode, Symbol};

use super::TestParser;

/// Parse direct calls and retain their object-local function relocations.
#[test]
fn test_parse_direct_call() {
    let (object, opcodes) = TestParser::new(
        r#"
external function double(int32): int32

export function apply(r0: int32): int32 {
    r1: int32 = call double(r0)
    return r1
}
"#,
    )
    .parse_opcodes(FunctionId(1));
    assert_eq!(opcodes, vec![Opcode::CALL, Opcode::RETURN]);
    assert_eq!(object.functions()[0].linkage, Linkage::EXTERNAL);
    assert_eq!(object.instruction_relocations().len(), 1);
    assert_eq!(
        object.instruction_relocations()[0].symbol,
        Symbol::function(0)
    );
}

/// Parse invoke control flow with explicit normal and unwind destinations.
#[test]
fn test_parse_invoke_edges() {
    let (_, opcodes) = TestParser::new(
        r#"
external function parse(int32): int32

export function checked(r0: int32): int32 {
    r1: int32 = invoke parse(r0) => l0 | l1

l0:
    return r1

l1:
    unwind.resume
}
"#,
    )
    .parse_opcodes(FunctionId(1));

    assert_eq!(
        opcodes,
        vec![Opcode::INVOKE, Opcode::RETURN, Opcode::UNWIND_RESUME]
    );
}
