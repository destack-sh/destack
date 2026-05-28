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
@if(1)
const value = 1;

"#,
        r#"
/// @diagnostic.error code=EC401 message="static condition requires boolean value"
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
let enabled = true;
/// @type.node source="let enabled = true" type=void
/// @type.symbol symbol=enabled type=boolean
/// @type.node source=true type=true

@if(enabled)
/// @resolution.name source=enabled target=enabled
/// @type.node source=enabled type=boolean

const value = 1;

"#,
        r#"
/// @diagnostic.error code=EC401 message="static condition requires boolean value"
/// @diagnostic.label line=4 column=5 source="@if(enabled)"
"#,
    );
}
