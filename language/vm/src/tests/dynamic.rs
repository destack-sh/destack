use destack_program::{DynamicEntry, FunctionId, TypeId, Word};

use super::{TestMachine, TestProgram};

/// Bind, inspect, and dispatch one erased dynamic value.
#[test]
fn test_execute_dynamic_value() {
    let program = TestProgram::words().dynamic_table(1, 0, []).dynamic_table(
        3,
        2,
        [DynamicEntry::function(FunctionId(0))],
    );
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.int32 r1, 41
    return r1
}

function f1 {
    dynamic.bind r1:r2, r0, d1
    extract r3, r1:r2, 0:8
    dynamic.type r4, r1:r2
    call.dynamic r5, r1:r2[0](r0)
    return r3:r5
}
"#,
        program,
    );

    let payload = Word::from_bits(0x2400);
    let concrete = Word::uint32(u32::from(TypeId(3)));
    let value = machine.complete(1, &[payload]);

    assert_eq!(value, vec![payload, concrete, Word::int32(41)]);
}
