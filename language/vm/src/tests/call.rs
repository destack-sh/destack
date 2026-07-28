use destack_mir::Space;
use destack_program::{BindingId, FunctionId, Word};

use super::{TestMachine, TestProgram};
use crate::{DiagnosticAnchor, Error};

/// Execute the Program implementation attached to one binding identity.
#[test]
fn test_execute_binding_definition() {
    let program = TestProgram::words().binding(0, "runtime.touch");
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.int32 r0, 42
    return r0
}
"#,
        program,
    );

    let value = machine.complete(0, &[]);

    assert_eq!(value, vec![Word::int32(42)]);
}

/// Report an unavailable runtime binding from every function entry path.
#[test]
fn test_report_unavailable_binding() {
    let binding = BindingId::from_static_name("runtime.touch");
    let program = TestProgram::words().binding(0, "runtime.touch");
    let mut machine = TestMachine::parse(
        r#"
function f0

function f1 {
    call r0, f0, _
    return r0
}

function f2 {
    tail.call f0, _
}
"#,
        program,
    );
    let function = FunctionId(0);
    let expected = Error::binding_unavailable(function, binding)
        .reason()
        .clone();

    // reject direct host entry without a linked Program implementation
    let error = machine
        .run(0, &[], None, None, None)
        .expect_err("bound entry should require a runtime implementation");
    assert_eq!(error.reason(), &expected);
    assert!(error.stack().is_empty());
    assert_eq!(error.anchor(), &DiagnosticAnchor::None);

    // reject ordinary and tail calls through the same code resolution path
    for caller in [1, 2] {
        let caller_id = FunctionId(caller);
        let error = machine
            .run(caller, &[], None, None, None)
            .expect_err("bound call should require a runtime implementation");
        assert_eq!(error.reason(), &expected);
        assert_eq!(error.stack().len(), 1);
        assert_eq!(error.stack()[0].function, caller_id);
        assert_eq!(
            error.anchor(),
            &DiagnosticAnchor::Point(TestProgram::point(caller_id.0, 0))
        );
    }
}

/// Dispatch through the table id initialized in one virtual object.
#[test]
fn test_execute_virtual_call() {
    let allocation = TestProgram::virtual_allocation(2, 0, Space::Local, 1, 1);
    let program = TestProgram::words()
        .allocations([allocation])
        .virtual_table(0, [0])
        .virtual_table(1, [1])
        .virtual_object(1, 16, 12);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.int32 r1, 7
    return r1
}

function f1 {
    constant.int32 r1, 42
    return r1
}

function f2 {
    new.local.managed.zeroed r0, a0
    call.virtual r1, r0, ref<managed, space(local)>, 12, 0, r0
    return r1
}
"#,
        program,
    );

    let value = machine.complete(2, &[]);

    assert_eq!(value, vec![Word::int32(42)]);
}
