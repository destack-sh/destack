use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow an if statement as the only statement in an else block.
    pub NO_LONELY_IF {
        id: "no-lonely-if",
        summary: "Disallow an if statement as the only statement in an else block",
        explanation: r#"
`else { if (condition) ... }` has the same control flow as `else if (condition) ...` when the block contains no other statements.
Instead, you SHOULD flatten the nested `if` into the existing conditional chain.
"#,
        example: {
            reported: r#"
function classify(value: int32): string {
    if (value > 0) {
        return "positive";
    } else {
        if (value < 0) {
            return "negative";
        }
    }
    return "zero";
}
"#,
            accepted: r#"
function classify(value: int32): string {
    if (value > 0) {
        return "positive";
    } else if (value < 0) {
        return "negative";
    }
    return "zero";
}
"#,
        },
        provenance: [Eslint("no-lonely-if")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report else blocks whose only statement is another if.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect regular if expressions with explicit else blocks
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form: dir::IfForm::If,
            else_expression: Some(else_expression),
            ..
        } = node
        else {
            continue;
        };

        // require one nested regular if and no other block expressions
        let dir::Expression::Block(block) = view.get(*else_expression) else {
            continue;
        };
        let Some(nested) = view.get(*block).only_expression() else {
            continue;
        };
        if !matches!(
            view.get(nested),
            dir::Expression::If {
                form: dir::IfForm::If,
                ..
            }
        ) {
            continue;
        }

        // replace the complete else clause when its nested if retains every comment
        let keyword = module.source_region(expression.into_any(), NodeSpanRegion::Else)?;
        let enclosing = module.source_extent(expression.into_any())?;
        let extent = module.source_extent((*else_expression).into_any())?;
        let extent = keyword.merge(extent);
        let mut diagnostic = lint.diagnostic("else block contains only another if", keyword);
        if let Some(suggestion) = suggestion(module, lint, enclosing, extent, nested)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one flattened else-if clause.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    enclosing: tspp_source::Span,
    extent: tspp_source::Span,
    nested: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let nested = module.source_extent(nested.into_any())?;
    if module.has_unretained_comment(extent, &[nested])? {
        return Ok(None);
    }

    // measure the indentation removed with the else block
    let file = module.file(extent.file)?;
    let (_, enclosing_column) = file.get_position(enclosing.start).ok_or_else(|| {
        ProviderError::internal("enclosing if extent is outside its authored source file")
    })?;
    let (_, nested_column) = file.get_position(nested.start).ok_or_else(|| {
        ProviderError::internal("nested if extent is outside its authored source file")
    })?;
    let indentation = nested_column
        .checked_sub(enclosing_column)
        .ok_or_else(|| ProviderError::internal("nested if begins before its enclosing if"))?;

    // retain and dedent the complete nested if expression
    let prefix = " ".repeat(indentation as usize);
    let nested_source = module.source(nested)?;
    let (first, remaining) = nested_source
        .split_once('\n')
        .map_or((nested_source, None), |(first, remaining)| {
            (first, Some(remaining))
        });
    let mut replacement = format!("else {first}");
    if let Some(remaining) = remaining {
        for line in remaining.split('\n') {
            let line = if line.is_empty() {
                line
            } else {
                let Some(line) = line.strip_prefix(&prefix) else {
                    return Ok(None);
                };

                line
            };
            replacement.push('\n');
            replacement.push_str(line);
        }
    }

    // replace the complete else clause
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("flatten the else-if chain", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Flatten an else block containing only another if.
    #[test]
    fn test_flattens_else_if_chain() {
        let session = TestSession::dir(
            &NO_LONELY_IF,
            r#"
function classify(value: int32): string {
    if (value > 0) {
        return "positive";
    } else {
        if (value < 0) {
            return "negative";
        }
    }
    return "zero";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-lonely-if]: else block contains only another if
 ──▶ main.tspp:4:7
  │
2 │     if (value > 0) {
3 │         return "positive";
4 │     } else {
  │       ^^^^
5 │         if (value < 0) {
6 │             return "negative";
  │

 = fix: flatten the else-if chain
--- a/main.tspp
+++ b/main.tspp

    3│         return "positive";
-   4│     } else {
-   5│         if (value < 0) {
-   6│             return "negative";
-   7│         }
+   4│     } else if (value < 0) {
+   5│         return "negative";
    8│     }
"#,
        );
        session.assert_fixes(
            r#"
function classify(value: int32): string {
    if (value > 0) {
        return "positive";
    } else if (value < 0) {
        return "negative";
    }
    return "zero";
}
"#,
        );
    }

    /// Accept an else block that performs another operation before its if.
    #[test]
    fn test_accepts_nontrivial_else_block() {
        let session = TestSession::dir(
            &NO_LONELY_IF,
            r#"
declare function record(): void;
function classify(value: int32): void {
    if (value > 0) {
        record();
    } else {
        record();
        if (value < 0) {
            record();
        }
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report without a fix when flattening would discard a comment.
    #[test]
    fn test_reports_commented_else_block_without_fix() {
        let session = TestSession::dir(
            &NO_LONELY_IF,
            r#"
function classify(value: int32): void {
    if (value > 0) {
    } else {
        // retain this branch note
        if (value < 0) {
        }
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-lonely-if]: else block contains only another if
 ──▶ main.tspp:3:7
  │
1 │ function classify(value: int32): void {
2 │     if (value > 0) {
3 │     } else {
  │       ^^^^
4 │         // retain this branch note
5 │         if (value < 0) {
  │
"#,
        );
    }
}
