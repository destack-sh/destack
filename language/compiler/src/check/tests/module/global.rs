use crate::tests::{DirRows, TestSession};

#[test]
fn test_configured_global_root_binding_resolves_without_import() {
    let session = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module(
            "globals.ds",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .module(
            "main.ds",
            r#"
const value = answer;
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["globals.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== globals.ds ===

=== annotated ===
global {
    const answer: int32 = 42;
}

=== checked ===
global {
    const answer: int32 = 42;
    /// @type.symbol symbol=answer type=int32
    /// @type.node source=42 type=int32

}

=== main.ds ===

=== annotated ===
const value: int32 = answer;

=== checked ===
const value = answer;
/// @type.symbol symbol=value type=int32
/// @type.node source=answer type=int32
/// @resolution.name source=answer target=globals.answer
"#,
    );
}
