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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function parse(status: "ready" | "error"): "ready" {
    let "ready" = status else {
        return "ready";
    };

    status
}

=== checked ===
function parse(status: "ready" | "error"): "ready" {
/// @type.symbol symbol=parse type=(status: "ready" | "error") => "ready"
/// @type.symbol symbol=status source="status: \"ready\" | \"error\"" type="ready" | "error"

    let "ready" = status else {
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source=status type="ready" | "error"
    /// @resolution.name source=status target=status

        return "ready";
        /// @type.node source="\"ready\"" type="ready"

    };

    status
    /// @type.node source=status type="ready"
    /// @resolution.name source=status target=status

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function parse(status: "ready" | "error"): int32 {
    let "ready" = status;
    1
}

=== checked ===
function parse(status: "ready" | "error"): int32 {
/// @type.symbol symbol=parse type=(status: "ready" | "error") => int32
/// @type.symbol symbol=status source="status: \"ready\" | \"error\"" type="ready" | "error"

    let "ready" = status;
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source=status type="ready" | "error"
    /// @resolution.name source=status target=status

    1
    /// @type.node source=1 type=int32

}
"#,
        r#"
/// @diagnostic.error code=EC406 message="refutable pattern in binding position: '\"error\"' is not covered"
/// @diagnostic.label line=3 column=9 source="\"ready\""
"#,
    );
}

#[test]
fn test_let_else_branch_cannot_complete_normally() {
    let session = TestSession::single(
        r#"
function parse(status: "ready" | "error"): int32 {
    let "ready" = status else {
        0
    };

    1
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function parse(status: "ready" | "error"): int32 {
    let "ready" = status else {
        0
    };

    1
}

=== checked ===
function parse(status: "ready" | "error"): int32 {
/// @type.symbol symbol=parse type=(status: "ready" | "error") => int32
/// @type.symbol symbol=status source="status: \"ready\" | \"error\"" type="ready" | "error"

    let "ready" = status else {
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source=status type="ready" | "error"
    /// @resolution.name source=status target=status

        0
        /// @type.node source=0 type=0

    };

    1
    /// @type.node source=1 type=int32

}
"#,
        r#"
/// @diagnostic.error code=EC416 message="else branch of let-else must diverge"
/// @diagnostic.label line=3 column=31 source="{\n        0\n    }"
"#,
    );
}
