use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Bind and invoke one closure through its two-word callable value.
#[test]
fn test_execute_function_value() {
    let mut machine = TestMachine::parse(
        r#"
function body(environment r0: ref<managed, space(local)>, r1: int32): (
    ref<managed, space(local)>,
    int32
) {
    r2: ref<managed, space(local)> = function.environment.current
    r3: int32 = move r1
    return r2, r3
}

function identity(r0: int32): (int32) {
    r1: int32 = move r0
    return r1
}

function captureless(environment r0: ref<managed, space(local)>, r1: int32): (int32) {
    r2: int32 = move r1
    return r2
}

export function apply(r0: ref<managed, space(local)>, r1: int32): (
    ref<managed, space(local)>,
    ref<managed, space(local)>,
    int32,
    fn,
    int32,
    int32
) {
    r2: function = function.bind body, r0
    r4: ref<managed, space(local)> = null
    r5: function = function.bind captureless, r4
    r7: int32 = call.indirect r5(r1)
    r8: ref<managed, space(local)> = function.environment r2
    r9: ref<managed, space(local)>, r10: int32 = call.indirect r2(r1)
    r11: fn = function.address identity
    r12: int32 = call.indirect r11(r1)
    r13: int32 = move r7
    return r8, r9, r10, r11, r12, r13
}
"#,
        TestProgram::new(),
    );

    let environment = Word::from_bits(0x1200);
    let identity = Word::from(machine.function_id("identity"));
    let value = machine.complete("apply", &[environment, Word::int32(73)]);

    assert_eq!(
        value,
        vec![
            environment,
            environment,
            Word::int32(73),
            identity,
            Word::int32(73),
            Word::int32(73)
        ]
    );
}
