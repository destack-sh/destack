use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow documentation comments without an attachment.
    pub NO_DETACHED_DOC_COMMENT {
        id: "no-detached-doc-comment",
        summary: "Disallow documentation comments without an attachment",
        explanation: "A documentation comment must be attached directly to the node it documents. Move detached documentation to its declaration or use an ordinary comment when the text describes the surrounding implementation rather than one declaration.",
        example: {
            reported: r#"
/// Retry behavior is configured by the caller.

function run(): void {}
"#,
            accepted: r#"
// Retry behavior is configured by the caller.

function run(): void {}
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report documentation comments outside visible documentation spans.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let blocks = module.comment_blocks()?;
    let mut output = LintOutput::default();

    // report each detached non-legal documentation block once
    for block in blocks {
        if !block.is_detached_documentation() || block.is_legal() {
            continue;
        }
        let patches = block.demote_documentation()?;
        let suggestion = lint.suggestion("use an ordinary comment", patches)?;
        let diagnostic = lint
            .diagnostic(
                "documentation comment is not attached to a node",
                block.span(),
            )
            .help("move the comment to a declaration or use an ordinary comment")
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept documentation attached directly to a declaration.
    #[test]
    fn test_accepts_attached_documentation() {
        let session = TestSession::dir(
            &NO_DETACHED_DOC_COMMENT,
            r#"
/// Run one operation.
/// Return after the operation completes.
function run(): void {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept documentation attached to a parameter.
    #[test]
    fn test_accepts_parameter_documentation() {
        let session = TestSession::dir(
            &NO_DETACHED_DOC_COMMENT,
            r#"
function run(/** The operation count. */ count: int32): void {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve detached legal documentation.
    #[test]
    fn test_accepts_detached_legal_documentation() {
        let session = TestSession::dir(
            &NO_DETACHED_DOC_COMMENT,
            r#"
/** @license MIT */

function run(): void {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report detached block documentation with an ordinary-comment suggestion.
    #[test]
    fn test_reports_detached_block_documentation() {
        let session = TestSession::dir(
            &NO_DETACHED_DOC_COMMENT,
            r#"
/** Explain the following operation. */

function run(): void {}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-detached-doc-comment]: documentation comment is not attached to a node
 ──▶ main.ds:1:1
  │
1 │ /** Explain the following operation. */
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │
3 │ function run(): void {}
  │

 = help: move the comment to a declaration or use an ordinary comment
 = suggestion: use an ordinary comment (requires review)
--- a/main.ds
+++ b/main.ds

-   1│ /** Explain the following operation. */
+   1│ /* Explain the following operation. */
"#,
        );
    }
}
