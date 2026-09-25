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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 3 = do {
    const base: 1 = 1;
    base + 2
};

=== dir ===
const value = do {
/// @type.symbol symbol=value source=value type=3
/// @resolution.pattern source=value kind=binding target=value
/// @type.node type=3

    const base = 1;
    /// @type.symbol symbol=base source=base type=1
    /// @resolution.pattern source=base kind=binding target=base
    /// @type.node source=1 type=1

    base + 2
    /// @type.node source="base + 2" type=3
    /// @type.node source=base type=1
    /// @resolution.name source=base target=base
    /// @resolution.operator source="base + 2" type=3 operator="+" kind=builtin operands=[base as 1 families=(integer), 2 as 2 families=(integer)]
    /// @resolution.place source=base placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=base root=base
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 2 = do {
    const scoped: 2 = 2;
    scoped
};

scoped;

=== dir ===
const value = do {
/// @type.symbol symbol=value source=value type=2
/// @resolution.pattern source=value kind=binding target=value
/// @type.node type=2

    const scoped = 2;
    /// @type.symbol symbol=scoped source=scoped type=2
    /// @resolution.pattern source=scoped kind=binding target=scoped
    /// @type.node source=2 type=2

    scoped
    /// @type.node source=scoped type=2
    /// @resolution.name source=scoped target=scoped
    /// @resolution.place source=scoped placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=scoped root=scoped

};

scoped;
/// @type.node source=scoped type=<error>
/// @resolution.unresolved source=scoped path=scoped
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'scoped'"
/// @diagnostic.label line=7 column=1 span="scoped" line_source="scoped;"
"#,
    );
}
