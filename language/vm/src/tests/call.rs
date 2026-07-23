use destack_mir::Space;
use destack_program::{BindingId, Word};

use super::{TestMachine, TestProgram};
use crate::{DiagnosticAnchor, Error};

/// Execute the Program implementation attached to one binding identity.
#[test]
fn test_execute_binding_definition() {
    let program = TestProgram::new().binding("touch", "runtime.touch");
    let mut machine = TestMachine::parse(
        r#"
export function touch(): int32 {
    r0: int32 = 42
    return r0
}
"#,
        program,
    );

    let value = machine.complete("touch", &[]);

    assert_eq!(value, vec![Word::int32(42)]);
}

/// Report an unavailable runtime binding from every function entry path.
#[test]
fn test_report_unavailable_binding() {
    let binding = BindingId::from_static_name("runtime.touch");
    let program = TestProgram::new().binding("touch", "runtime.touch");
    let mut machine = TestMachine::parse(
        r#"
external function touch(): int32

export function callTouch(): int32 {
    r0: int32 = call touch()
    return r0
}

export function tailTouch(): int32 {
    tail.call touch()
}
"#,
        program,
    );
    let function = machine.function_id("touch");
    let expected = Error::binding_unavailable(function, binding).reason;

    // reject direct host entry without a linked Program implementation
    let error = machine
        .run("touch", &[], None, None, None)
        .expect_err("bound entry should require a runtime implementation");
    assert_eq!(error.reason, expected);
    assert!(error.stack.is_empty());
    assert_eq!(error.anchor, DiagnosticAnchor::None);

    // reject ordinary and tail calls through the same code resolution path
    for caller in ["callTouch", "tailTouch"] {
        let caller_id = machine.function_id(caller);
        let error = machine
            .run(caller, &[], None, None, None)
            .expect_err("bound call should require a runtime implementation");
        assert_eq!(error.reason, expected);
        assert_eq!(error.stack.len(), 1);
        assert_eq!(error.stack[0].function, caller_id);
        assert_eq!(
            error.anchor,
            DiagnosticAnchor::Point(TestMachine::point(caller_id.0, 0))
        );
    }
}

/// Dispatch through the table id initialized in one virtual object.
#[test]
fn test_execute_virtual_call() {
    let allocation = TestMachine::virtual_allocation(2, 0, Space::Local, 1, 1);
    let program = TestProgram::new()
        .allocations([allocation])
        .virtual_table(0, [0])
        .virtual_table(1, [1])
        .virtual_object(1, 16, 12);
    let mut machine = TestMachine::parse(
        r#"
type Dummy
type Concrete

function wrong(r0: ref<managed, space(local)>): int32 {
    r1: int32 = 7
    return r1
}

function right(r0: ref<managed, space(local)>): int32 {
    r1: int32 = 42
    return r1
}

export function apply(): int32 {
    r0: ref<managed, space(local)> = new.local.managed.zeroed Concrete
    r1: int32 = call.virtual r0, dispatch 12, slot 0(r0)
    return r1
}
"#,
        program,
    );

    let value = machine.complete("apply", &[]);

    assert_eq!(value, vec![Word::int32(42)]);
}
