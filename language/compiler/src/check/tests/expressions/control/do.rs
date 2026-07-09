use crate::tests::{DirRows, TestSession};

#[test]
fn test_do_expression_uses_tail_expression_value() {
    let session = TestSession::single(
        r#"
const value = do {
    const base = 1;
    base + 2
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 3 = do {
    const base: 1 = 1;
    base + 2
};

=== checked ===
const value = do {
/// @type.symbol symbol=value source=value type=3
/// @type.node type=3

    const base = 1;
    /// @type.symbol symbol=base source=base type=1
    /// @type.node source=1 type=1

    base + 2
    /// @type.node source="base + 2" type=3
    /// @type.node source=base type=1
    /// @resolution.name source=base target=base
    /// @resolution.call source="base + 2" parameters=() return=3 kind=builtin builtin=binary.add
    /// @type.node source=2 type=2

};
"#,
    );
}

#[test]
fn test_do_expression_keeps_local_bindings_scoped() {
    let session = TestSession::single(
        r#"
const value = do {
    const scoped = 2;
    scoped
};

scoped;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 2 = do {
    const scoped: 2 = 2;
    scoped
};

scoped;

=== checked ===
const value = do {
/// @type.symbol symbol=value source=value type=2
/// @type.node type=2

    const scoped = 2;
    /// @type.symbol symbol=scoped source=scoped type=2
    /// @type.node source=2 type=2

    scoped
    /// @type.node source=scoped type=2
    /// @resolution.name source=scoped target=scoped

};

scoped;
/// @type.node source=scoped type=<error>
"#,
        r#"
/// @diagnostic.error code=EC308 message="cannot find 'scoped'"
/// @diagnostic.label line=7 column=1 span="scoped" line_source="scoped;"
"#,
    );
}
