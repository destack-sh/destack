use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_else_literal_pattern_narrows_after_success() {
    let session = TestSession::single(
        r#"
function parse(status: "ready" | "error"): "ready" {
    let "ready" = status else {
        return "ready";
    };

    status
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function parse(status: "ready" | "error"): "ready" {
    let "ready" = status else {
        return "ready";
    };

    status
}

=== dir ===
function parse(status: "ready" | "error"): "ready" {
/// @type.symbol symbol=parse type=("ready" | "error") => "ready"
/// @type.symbol symbol=parse.status source="status: \"ready\" | \"error\"" type="ready" | "error"

    let "ready" = status else {
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source=status type="ready" | "error"
    /// @resolution.name source=status target=parse.status
    /// @resolution.access source=status root=parse.status

        return "ready";
        /// @type.node source="\"ready\"" type="ready"

    };

    status
    /// @type.node source=status type="ready"
    /// @resolution.name source=status target=parse.status
    /// @resolution.place source=status placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=status root=parse.status
    /// @resolution.narrowing source=status union="ready" | "error" arms="ready"

}
"#,
    );
}

#[test]
fn test_refutable_let_pattern_reports_missing_else() {
    let session = TestSession::single(
        r#"
function parse(status: "ready" | "error"): int32 {
    let "ready" = status;
    1
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function parse(status: "ready" | "error"): int32 {
    let "ready" = status;
    1
}

=== dir ===
function parse(status: "ready" | "error"): int32 {
/// @type.symbol symbol=parse type=("ready" | "error") => int32
/// @type.symbol symbol=parse.status source="status: \"ready\" | \"error\"" type="ready" | "error"

    let "ready" = status;
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source=status type="ready" | "error"
    /// @resolution.name source=status target=parse.status
    /// @resolution.access source=status root=parse.status

    1
    /// @type.node source=1 type=1

}
"#,
        r#"
/// @diagnostic.error id=refutable-pattern message="refutable pattern in binding position: '\"error\"' is not covered"
/// @diagnostic.label line=3 column=9 span="\"ready\"" line_source="let \"ready\" = status;"
/// @diagnostic.help message="handle the uncovered values with 'if let' or 'match'"
"#,
    );
}

#[test]
fn test_let_else_branch_cannot_complete_normally() {
    let session = TestSession::single(
        r#"
function parse(status: "ready" | "error"): int32 {
    let "ready" = status else {
        0;
    };

    1
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function parse(status: "ready" | "error"): int32 {
    let "ready" = status else {
        0;
    };

    1
}

=== dir ===
function parse(status: "ready" | "error"): int32 {
/// @type.symbol symbol=parse type=("ready" | "error") => int32
/// @type.symbol symbol=parse.status source="status: \"ready\" | \"error\"" type="ready" | "error"

    let "ready" = status else {
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source=status type="ready" | "error"
    /// @resolution.name source=status target=parse.status
    /// @resolution.access source=status root=parse.status

        0;
        /// @type.node source=0 type=0

    };

    1
    /// @type.node source=1 type=1

}
"#,
        r#"
/// @diagnostic.error id=let-else-branch-can-complete message="else branch of let-else must diverge"
/// @diagnostic.label line=3 column=31 span="{\n        0;\n    }" line_source="let \"ready\" = status else {"
"#,
    );
}
