use tspp_program::Word;

use super::{TestMachine, TestProgram};

/// Bind and invoke one closure through its two-word callable value.
#[test]
fn test_execute_function_value() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    move r2, r0
    move r3, r1
    return r2:r3
}

function f1 {
    move r1, r0
    return r1
}

function f2 {
    move r2, r1
    return r2
}

function f3 {
    function.bind r2:r3, f0, r0
    constant.null r4
    function.bind r5:r6, f2, r4
    call.indirect r7, r5:r6(r1)
    extract r8, r2:r3, 8:8
    call.indirect r9:r10, r2:r3(r1)
    function.address r11, f1
    call.indirect r12, r11(r1)
    move r13, r7
    return r8:r13
}
"#,
        TestProgram::words(),
    );

    let environment = Word::from_bits(0x1200);
    let identity = Word::from(tspp_program::FunctionId(1));
    let value = machine.complete(3, &[environment, Word::int32(73)]);

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
