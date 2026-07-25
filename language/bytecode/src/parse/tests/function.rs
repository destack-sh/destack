use crate::{Coroutine, FunctionId, Opcode, RegisterId, RegisterSpan, RelocationTag, TypeId};

use super::TestParser;

/// Parse closure binding, physical environment extraction, and indirect calls.
#[test]
fn test_parse_function_values() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0(): t0 {    return r1
}

function f1(): t0 {    function.bind r2:r3, f0, r0
    extract r4, r2:r3, 8, 8
    call.indirect r5, r2:r3, r1
    return r5
}
"#,
    )
    .parse_opcodes(FunctionId(1));

    assert_eq!(
        opcodes,
        vec![
            Opcode::FUNCTION_BIND,
            Opcode::EXTRACT,
            Opcode::CALL_INDIRECT,
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
}

/// Parse all TS-compatible coroutine function modifiers.
#[test]
fn test_parse_function_modifiers() {
    let object = TestParser::new(
        r#"
function regular(): t0

async function task(r0: t1, r1:r2: t2): t3

function* generate(): t0

async function* stream(): t0
"#,
    )
    .parse();
    let coroutines = object
        .functions()
        .iter()
        .map(|function| function.coroutine)
        .collect::<Vec<_>>();

    assert_eq!(
        coroutines,
        vec![
            Coroutine::NONE,
            Coroutine::ASYNC,
            Coroutine::GENERATOR,
            Coroutine::ASYNC_GENERATOR,
        ]
    );
    let task = &object.functions()[1];
    let parameters = task.parameters(object.parameters());

    assert_eq!(
        parameters
            .iter()
            .map(|parameter| (parameter.registers, parameter.ty))
            .collect::<Vec<_>>(),
        vec![
            (RegisterSpan::new(RegisterId(0), 1), TypeId(1)),
            (RegisterSpan::new(RegisterId(1), 2), TypeId(2)),
        ]
    );
    assert_eq!(task.result, TypeId(3));
}
