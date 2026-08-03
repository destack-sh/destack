use crate::{FunctionId, Opcode, RelocationTag};

use super::TestParser;

/// Parse direct calls and retain their object-local function relocations.
#[test]
fn test_parse_direct_call() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0 {    call r1, f1, r0
    return r1
}

external function f1
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(opcodes, vec![Opcode::CALL, Opcode::RETURN]);
    assert!(object.functions()[1].code().is_none());
    assert_eq!(object.relocations()[0].tag, RelocationTag::FUNCTION);
}

/// Parse invoke control flow with explicit normal and unwind destinations.
#[test]
fn test_parse_invoke_edges() {
    let (_, opcodes) = TestParser::new(
        r#"
external function f0
function f1 {    invoke r1, f0, r0 => b0 | b1

b0:
    return r1

b1:
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
