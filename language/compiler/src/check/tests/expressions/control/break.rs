use crate::tests::{DirRows, TestSession};

#[test]
fn test_nested_function_break_reports_error() {
    let session = TestSession::single(
        r#"
while (true) {
    const stop = () => {
        break;
    };
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
while (true) {
    const stop: Function<(), void> = (): void => {
        break;
    };
}

=== checked ===
while (true) {
/// @type.node source=true type=true

    const stop = () => {
    /// @type.symbol symbol=stop source=stop type=Function<(), void>
    /// @type.symbol symbol=symbol1 type=Function<(), void>
    /// @type.node type=Function<(), void>

        break;
        /// @type.node source=break type=never

    };
}
"#,
        r#"
/// @diagnostic.error code=EC402 message="break statement has no target"
/// @diagnostic.label line=4 column=9 source="break;"
"#,
    );
}
