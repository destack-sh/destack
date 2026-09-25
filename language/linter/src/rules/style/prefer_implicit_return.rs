use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer expression bodies for lambdas that only return one value.
    pub PREFER_IMPLICIT_RETURN {
        id: "prefer-implicit-return",
        summary: "Prefer expression bodies for lambdas that only return one value",
        explanation: r#"
A lambda block containing only `return value` adds a statement around its result.
Instead, you SHOULD use `value` as the lambda body directly.
"#,
        example: {
            reported: r#"
function double(values: int32[]): int32[] {
    return values.map((value) => {
        return value * 2;
    });
}
"#,
            accepted: r#"
function double(values: int32[]): int32[] {
    return values.map((value) => value * 2);
}
"#,
        },
        provenance: [Eslint("arrow-body-style")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report lambda blocks whose only statement returns one value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect lambda declarations with explicit block bodies
    for (declaration, node) in view.iter_nodes::<dir::Declaration>() {
        let dir::Declaration::Function(function) = node else {
            continue;
        };
        if function.signature.form != dir::FunctionForm::Lambda || function.signature.is_generator {
            continue;
        }
        let Some(body) = function.body else {
            continue;
        };
        if !matches!(view.get(body), dir::Expression::Block(_)) {
            continue;
        }
        let Some(value) = module.sole_return_value(body) else {
            continue;
        };

        // replace the complete block body when every comment is retained
        let span = module.source_extent(body.into_any())?;
        let mut diagnostic = lint.diagnostic("lambda only returns one expression", span);
        if let Some(suggestion) = suggestion(module, lint, declaration, body, value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one expression-body replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    declaration: dir::LocalNodeId<dir::Declaration>,
    body: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(body.into_any())?;
    let retained = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain grouping required for object literal lambda bodies
    let source = module.source(retained)?;
    let replacement = if matches!(
        module.view().get(value),
        dir::Expression::ObjectExpression { .. }
    ) {
        format!("({source})")
    } else {
        source.to_string()
    };
    let body = module.source_region(declaration.into_any(), NodeSpanRegion::Body)?;
    let patch = Patch::replace(body, replacement);
    let suggestion = lint.suggestion("use an expression body", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Parenthesize an object literal used as the expression body.
    #[test]
    fn test_parenthesizes_object_literal() {
        let session = TestSession::dir(
            &PREFER_IMPLICIT_RETURN,
            r#"
const wrap = (value: int32) => {
    return { value };
};
"#,
        );

        session.assert_suggestions(
            r#"
const wrap = (value: int32) => ({ value });
"#,
        );
    }

    /// Accept a lambda block that performs work before returning.
    #[test]
    fn test_accepts_multiple_statements() {
        let session = TestSession::dir(
            &PREFER_IMPLICIT_RETURN,
            r#"
const double = (value: int32): int32 => {
    value;
    return value * 2;
};
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report without a suggestion when removing the block would discard a comment.
    #[test]
    fn test_reports_commented_block_without_suggestion() {
        let session = TestSession::dir(
            &PREFER_IMPLICIT_RETURN,
            r#"
const double = (value: int32): int32 => {
    // keep this explanation
    return value * 2;
};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-implicit-return]: lambda only returns one expression
 ──▶ main.tspp:1:41
  │
1 │ const double = (value: int32): int32 => {
  │                                         ^
2 │     // keep this explanation
  │     ^^^^^^^^^^^^^^^^^^^^^^^^
3 │     return value * 2;
  │     ^^^^^^^^^^^^^^^^^
4 │ };
  │ ^
  │
"#,
        );
    }
}
