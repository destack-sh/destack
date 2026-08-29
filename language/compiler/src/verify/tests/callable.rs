use crate::tests::TestSession;

/// Preserve repeatable callable values across calls for every environment form.
#[test]
fn test_call_repeatable_functions_more_than_once() {
    let session = TestSession::single(
        r#"
function invokeManaged(run: Function<(), void>): void {
    run();
    run();
}

function invokeOwned(run: ^Function<(), void>): void {
    run();
    run();
}

function invokeBorrowed(run: &Function<(), void>): void {
    run();
    run();
}

function invokeExclusive(run: &exclusive Function<(), void>): void {
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

/// Consume an owned once callable on its first call.
#[test]
fn test_call_owned_once_function_once() {
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

/// Reject a second call of an owned once callable.
#[test]
fn test_reject_calling_owned_once_function_more_than_once() {
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
