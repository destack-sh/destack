use crate::tests::TestSession;

/// Preserve callable values with borrowed receivers across calls for every environment form.
#[test]
fn test_call_callables_with_borrowed_receivers_more_than_once() {
    let session = TestSession::single(
        r#"
function invokeManaged(run: Function<(), void, "readonly">): void {
    run();
    run();
}

function invokeOwned(run: ^Function<(), void, "mutable">): void {
    run();
    run();
}

function invokeBorrowed(run: &Function<(), void, "readonly">): void {
    run();
    run();
}

function invokeExclusive(run: &Function<(), void, "mutable">): void {
    run();
    run();
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// Consume an owned callable on its first call.
#[test]
fn test_call_an_owned_callable_once() {
    let session = TestSession::single(
        r#"
function invokeOwned(run: ^Function<(), void, "once">): void {
    run();
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// Reject a second call of an owned callable.
#[test]
fn test_reject_calling_an_owned_callable_more_than_once() {
    let session = TestSession::single(
        r#"
function invokeOwned(run: ^Function<(), void, "once">): void {
    run();
    run();
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=use-after-move message="use of moved value"
/// @diagnostic.label line=4 column=5 span="run()" line_source="run();"
/// @diagnostic.related line=3 column=5 span="run()" line_source="run();" message="value moved here"
"#,
    );
}

/// Release a once closure's environment after its captures move out.
#[test]
fn test_release_a_once_closure_environment_after_its_captures_move_out() {
    let session = TestSession::single(
        r#"
import { Box } from "destack:memory";

function invokeOwned(run: ^Function<(), void, "once">): void {
    run();
}

function consume(value: ^Box<int32>): void {}

function spawn(): void {
    let value = Box.new(1);
    @capture("move")
    let run: ^Function<(), void, "once"> = () => {
        consume(value);
    };
    invokeOwned(run);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"

"#,
    );
}

/// Read a copyable value through a closure parameter borrow under the enclosing where clause.
#[test]
fn test_copy_through_a_closure_parameter_borrow_under_an_enclosing_where_clause() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

function keep<T>(predicate: (value: T) => boolean): ((value: &immutable T) => boolean) where T: Copy {
    return (value: &immutable T) => predicate(*value);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}
