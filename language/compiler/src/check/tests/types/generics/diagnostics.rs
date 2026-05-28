use crate::tests::{DirRows, TestSession};

#[test]
fn test_explicit_type_argument_mismatch_reports_assignability_error() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

identity<int32>("x");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

identity<int32>("x");
/// @resolution.name source=identity target=identity
/// @type.node source=identity type=<T>(T) => T
/// @type.node source="\"x\"" type="x"

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=6 column=17 source="identity<int32>(\"x\");"
"#,
    );
}
