use crate::tests::{DirRows, TestSession};

#[test]
fn test_static_if_non_boolean_condition_reports_error() {
    let session = TestSession::single(
        r#"
@if(1)
const value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@if(1)
const value = 1;

=== checked ===
@if(1)
const value = 1;

"#,
        r#"
/// @diagnostic.error code=EC401 message="static condition must evaluate to a boolean"
/// @diagnostic.label line=2 column=5 source="@if(1)"
"#,
    );
}

#[test]
fn test_static_if_runtime_condition_reports_error() {
    let session = TestSession::single(
        r#"
let enabled = true;

@if(enabled)
const value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let enabled: boolean = true;

@if(enabled)
const value: 1 = 1;

=== checked ===
let enabled = true;
/// @type.symbol symbol=enabled source=enabled type=boolean
/// @type.node source=true type=true

@if(enabled)
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1

"#,
        r#"
/// @diagnostic.error code=EC401 message="static condition requires boolean value"
/// @diagnostic.label line=4 column=5 source="@if(enabled)"
"#,
    );
}
