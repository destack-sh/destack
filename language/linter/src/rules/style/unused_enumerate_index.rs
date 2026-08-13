use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow Array.entries iteration when its index is unused.
    pub UNUSED_ENUMERATE_INDEX {
        id: "unused-enumerate-index",
        summary: "Disallow Array.entries iteration when its index is unused",
        explanation: r#"
Iterating over Array.entries constructs an index-value pair for every element even when the index is unused.
Instead, you SHOULD iterate over the array values directly.
"#,
        example: {
            reported: r#"
function copy(values: int32[], output: int32[]): void {
    for (const (_, value) of values.entries()) {
        output.push(value);
    }
}
"#,
            accepted: r#"
function copy(values: int32[], output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Reject this lint until DIR carries checked flow uses.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} requires checked flow uses in DIR",
        lint.id
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Validate the canonical lint example.
    #[test]
    fn test_lint_example() {
        TestSession::assert_example(&UNUSED_ENUMERATE_INDEX);
    }

    /// Remove entries when a wildcard discards the index.
    #[test]
    fn test_removes_wildcard_index() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
function copy(values: int32[], output: int32[]): void {
    for (const (_, value) of values.entries()) {
        output.push(value);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[unused-enumerate-index]: Array.entries index is unused
 ──▶ main.ds:2:17
  │
1 │ function copy(values: int32[], output: int32[]): void {
2 │     for (const (_, value) of values.entries()) {
  │                 ^
3 │         output.push(value);
4 │     }
  │

 = fix: iterate over array values directly
--- a/main.ds
+++ b/main.ds

    1│ function copy(values: int32[], output: int32[]): void {
-   2│     for (const (_, value) of values.entries()) {
+   2│     for (const value of values) {
"#,
        );
        session.assert_fixes(UNUSED_ENUMERATE_INDEX.example.accepted());
    }

    /// Remove entries when a named index has no references.
    #[test]
    fn test_removes_unused_named_index() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
function copy(values: int32[], output: int32[]): void {
    for (const (index, value) of values.entries()) {
        output.push(value);
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function copy(values: int32[], output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        );
    }

    /// Accept entries when the index is referenced.
    #[test]
    fn test_accepts_used_index() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
function copy(values: int32[], indexes: usize[], output: int32[]): void {
    for (const (index, value) of values.entries()) {
        indexes.push(index);
        output.push(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined entries method.
    #[test]
    fn test_accepts_user_entries() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
class Values {
    entries(): (usize, int32)[] {
        return [];
    }
}

function visit(values: Values): void {
    for (const (_, value) of values.entries()) {
        // intentionally empty
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
