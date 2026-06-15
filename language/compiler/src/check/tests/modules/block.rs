use crate::tests::{DirRows, TestSession};

#[test]
fn test_module_block_declaration_warns_pointless() {
    let session = TestSession::single(
        r#"
module {
    const role = "server";
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
module {
    const role: "server" = "server";
}

=== checked ===
module {
    const role = "server";
    /// @type.symbol symbol=role source=role type="server"

}
"#,
        r#"
/// @diagnostic.warning code=WC106 message="pointless declaration in module block"
/// @diagnostic.label line=3 column=5 source="const role = \"server\";"
"#,
    );
}
