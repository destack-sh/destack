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

function invokeExclusive(run: &exclusive Function<(), void, "exclusive">): void {
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
