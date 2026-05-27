use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_rejects_break_from_nested_function() {
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
while (true) {
/// @type.node type=void
/// @type.node source=true type=true

    const stop = () => {
    /// @type.node type=void
    /// @type.symbol symbol=stop type=() => void
    /// @type.symbol symbol=symbol1 type=() => void
    /// @type.node type=() => void
    /// @type.node type=void

        break;
        /// @type.node source=break type=never

    };
}
"#,
        r#"
/// @diagnostic.error code=EC402 message="invalid control flow: break has no target"
/// @diagnostic.label line=4 column=9 source="break;"
"#,
    );
}
