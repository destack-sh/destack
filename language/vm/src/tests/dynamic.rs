use destack_program::{DynamicEntry, FunctionId, Word};

use super::{TestMachine, TestProgram};

/// Bind, inspect, and dispatch one erased dynamic value.
#[test]
fn test_execute_dynamic_value() {
    let program = TestProgram::new().dynamic_table(1, 0, []).dynamic_table(
        3,
        2,
        [DynamicEntry::function(FunctionId(0))],
    );
    let mut machine = TestMachine::parse(
        r#"
type DummyConstraint
type DummyConcrete
type Constraint
type Concrete

function method(r0: ref<managed, space(local)>): int32 {
    r1: int32 = 41
    return r1
}

export function apply(r0: ref<managed, space(local)>): (
    ref<managed, space(local)>,
    typeId,
    int32
) {
    r1: dynamic<Constraint, space(local)> = dynamic.bind r0: Concrete
    r3: ref<managed, space(local)> = dynamic.payload r1
    r4: typeId = dynamic.type r1
    r5: int32 = call.dynamic r1, slot 0(r0)
    return r3, r4, r5
}
"#,
        program,
    );

    let payload = Word::from_bits(0x2400);
    let concrete = Word::uint32(u32::from(machine.type_id("Concrete")));
    let value = machine.complete("apply", &[payload]);

    assert_eq!(value, vec![payload, concrete, Word::int32(41)]);
}
