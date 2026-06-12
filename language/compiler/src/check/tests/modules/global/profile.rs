use crate::tests::{DirRows, TestSession};

#[test]
fn test_profile_global_binding_resolves_as_name() {
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
global {
    const answer: int32 = 42;
    /// @type.symbol symbol=answer type=int32
    /// @type.node source=42 type=int32

}

=== main.ds ===
const value = answer;
/// @type.symbol symbol=value type=int32
/// @type.node source=answer type=int32
/// @resolution.name source=answer target=globals.answer
"#,
    );
}
