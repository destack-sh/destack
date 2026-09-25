use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow direct borrows that are immediately dereferenced.
    pub NEEDLESS_BORROW {
        id: "needless-borrow",
        summary: "Disallow direct borrows that are immediately dereferenced",
        explanation: r#"
Dereferencing a borrow created by the same expression returns the original value place.
Instead, you SHOULD use that value directly.
"#,
        example: {
            reported: r#"
function identity(value: int32): int32 {
    return *&value;
}
"#,
            accepted: r#"
function identity(value: int32): int32 {
    return value;
}
"#,
        },
        provenance: [Clippy("needless_borrow")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report direct dereferences of a borrow authored at the same expression.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect explicit dereference expressions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Unary {
            operator: dir::UnaryOperator::Dereference,
            right,
        } = node
        else {
            continue;
        };

        // require a borrow authored directly beneath the dereference
        let dir::Expression::BorrowOf { right: value, .. } = view.get(*right) else {
            continue;
        };

        // require compiler-defined dereference on every selected arm
        let Some(resolution) = module.operator_decision(expression.into_any())? else {
            continue;
        };
        let is_direct = resolution
            .arms()
            .iter()
            .all(dir::OperatorApplication::is_builtin);
        if !is_direct {
            continue;
        }

        // replace the complete borrow and dereference with its original value
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("borrow is immediately dereferenced", span);
        if let Some(suggestion) = suggestion(module, lint, expression, *value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the exact original-value replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let mut extent = module.source_extent(expression.into_any())?;
    let dir::Expression::Unary { right: borrow, .. } = module.view().get(expression) else {
        return Err(ProviderError::internal(format!(
            "needless-borrow candidate {} is not a unary expression",
            expression.id
        )));
    };
    if let Some(parentheses) = module.source_parentheses(borrow.into_any()) {
        extent = extent.merge(parentheses);
    }
    let value_extent = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[value_extent])? {
        return Ok(None);
    }

    // retain authored grouping around the borrowed expression
    let replacement = module.expression_source(value, dir::OperatorPrecedence::Prefix)?;
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use the original value", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Preserve grouping when replacing a borrowed binary expression.
    #[test]
    fn test_replaces_borrowed_binary_expression() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW,
            r#"
function sum(left: int32, right: int32): int32 {
    return *(&(left + right));
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-borrow]: borrow is immediately dereferenced
 ──▶ main.tspp:2:12
  │
1 │ function sum(left: int32, right: int32): int32 {
2 │     return *(&(left + right));
  │            ^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the original value
--- a/main.tspp
+++ b/main.tspp

    1│ function sum(left: int32, right: int32): int32 {
-   2│     return *(&(left + right));
+   2│     return (left + right);
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function sum(left: int32, right: int32): int32 {
    return (left + right);
}
"#,
        );
    }

    /// Accept dereferencing a borrow received from another expression.
    #[test]
    fn test_accepts_stored_borrow() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW,
            r#"
function read(value: &readonly int32): int32 {
    return *value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comments inside a direct borrow by omitting the fix.
    #[test]
    fn test_reports_commented_borrow_without_fix() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW,
            r#"
function read(value: int32): int32 {
    return *(&/* retain */ value);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-borrow]: borrow is immediately dereferenced
 ──▶ main.tspp:2:12
  │
1 │ function read(value: int32): int32 {
2 │     return *(&/* retain */ value);
  │            ^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
