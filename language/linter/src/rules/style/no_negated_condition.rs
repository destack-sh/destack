use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer positive conditions when both branches are present.
    pub NO_NEGATED_CONDITION {
        id: "no-negated-condition",
        summary: "Prefer positive conditions when both branches are present",
        explanation: r#"
A negated condition assigns the positive case to the second of two explicit branches.
Instead, you SHOULD remove the negation and exchange the branches.
"#,
        example: {
            reported: r#"
function status(isReady: boolean): string {
    return !isReady ? "waiting" : "ready";
}
"#,
            accepted: r#"
function status(isReady: boolean): string {
    return isReady ? "ready" : "waiting";
}
"#,
        },
        provenance: [Eslint("no-negated-condition")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One authored negation that can be stated positively.
#[derive(Debug, Clone, Copy)]
enum Negation {
    /// Prefix boolean negation.
    Not(dir::LocalNodeId<dir::Expression>),
    /// Builtin loose inequality.
    LooseInequality,
    /// Builtin strict inequality.
    StrictInequality,
}

impl Negation {
    /// Select one builtin negation.
    fn select(
        module: &DirModule<'_>,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        let negation = match module.view().get(condition) {
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right,
            } if module.builtin_unary(condition)?.is_some() => Self::Not(*right),
            dir::Expression::Binary {
                operator: dir::BinaryOperator::NotEqual,
                ..
            } if module.builtin_binary(condition)?.is_some() => Self::LooseInequality,
            dir::Expression::Binary {
                operator: dir::BinaryOperator::NotEqualStrict,
                ..
            } if module.builtin_binary(condition)?.is_some() => Self::StrictInequality,
            _ => return Ok(None),
        };

        Ok(Some(negation))
    }
}

/// Report two-way branches controlled by builtin boolean negation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect complete branches with one expression condition
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form,
            condition,
            then_expression,
            else_expression: Some(else_expression),
        } = node
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };

        // preserve the head of an authored else-if chain
        if *form == dir::IfForm::If
            && matches!(
                view.get(*else_expression),
                dir::Expression::If {
                    form: dir::IfForm::If,
                    ..
                }
            )
        {
            continue;
        }

        // select builtin prefix or inequality negation
        let Some(negation) = Negation::select(module, condition)? else {
            continue;
        };

        // exchange ternary branches when the authored source can be retained
        let span = module.source_extent(condition.into_any())?;
        let mut diagnostic = lint.diagnostic("two-way branch uses a negated condition", span);
        if *form == dir::IfForm::Ternary
            && let Some(suggestion) = suggestion(
                module,
                lint,
                expression,
                condition,
                negation,
                *then_expression,
                *else_expression,
            )?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one positive ternary with exchanged branches.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    condition: dir::LocalNodeId<dir::Expression>,
    negation: Negation,
    then_expression: dir::LocalNodeId<dir::Expression>,
    else_expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let condition_extent = module.source_extent(condition.into_any())?;
    let then_extent = module.source_extent(then_expression.into_any())?;
    let else_extent = module.source_extent(else_expression.into_any())?;
    let retained_condition = match negation {
        Negation::Not(value) => module.source_extent(value.into_any())?,
        Negation::LooseInequality | Negation::StrictInequality => condition_extent,
    };
    if module.has_unretained_comment(extent, &[retained_condition, then_extent, else_extent])? {
        return Ok(None);
    }

    // state the condition positively
    let mut file = FilePatch::new(extent.file);
    match negation {
        Negation::Not(value) => {
            let value = module.expression_source(value, dir::OperatorPrecedence::TypeRelation)?;

            file.replace(condition_extent, value);
        }
        Negation::LooseInequality => {
            file.replace(module.main_span(condition.into_any())?, "==");
        }
        Negation::StrictInequality => {
            file.replace(module.main_span(condition.into_any())?, "===");
        }
    }

    // exchange the two branch expressions
    let then_source = module.source(then_extent)?;
    let else_source = module.source(else_extent)?;
    file.replace(then_extent, else_source);
    file.replace(else_extent, then_source);
    file.sort();
    let suggestion = lint.fix("use the positive condition", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a negated condition with no alternative branch.
    #[test]
    fn test_accepts_one_way_negated_condition() {
        let session = TestSession::dir(
            &NO_NEGATED_CONDITION,
            r#"
function requireReady(isReady: boolean): void {
    if (!isReady) {
        return;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept negation at the head of an else-if chain.
    #[test]
    fn test_accepts_negated_else_if_chain() {
        let session = TestSession::dir(
            &NO_NEGATED_CONDITION,
            r#"
function status(isReady: boolean, isWaiting: boolean): string {
    if (!isReady) {
        return "stopped";
    } else if (isWaiting) {
        return "waiting";
    }
    return "ready";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace builtin inequality with equality and exchange ternary branches.
    #[test]
    fn test_replaces_inequality_condition() {
        let session = TestSession::dir(
            &NO_NEGATED_CONDITION,
            r#"
function status(left: int32, right: int32): string {
    return left !== right ? "different" : "same";
}
"#,
        );

        session.assert_fixes(
            r#"
function status(left: int32, right: int32): string {
    return left === right ? "same" : "different";
}
"#,
        );
    }

    /// Replace builtin loose inequality with equality and exchange ternary branches.
    #[test]
    fn test_replaces_loose_inequality_condition() {
        let session = TestSession::dir(
            &NO_NEGATED_CONDITION,
            r#"
function status(left: int32, right: int32): string {
    return left != right ? "different" : "same";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-negated-condition]: two-way branch uses a negated condition
 ──▶ main.tspp:2:12
  │
1 │ function status(left: int32, right: int32): string {
2 │     return left != right ? "different" : "same";
  │            ^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the positive condition
--- a/main.tspp
+++ b/main.tspp

    1│ function status(left: int32, right: int32): string {
-   2│     return left != right ? "different" : "same";
+   2│     return left == right ? "same" : "different";
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function status(left: int32, right: int32): string {
    return left == right ? "same" : "different";
}
"#,
        );
    }

    /// Report a negated statement condition without offering a structural rewrite.
    #[test]
    fn test_reports_negated_if_condition_without_fix() {
        let session = TestSession::dir(
            &NO_NEGATED_CONDITION,
            r#"
function status(isReady: boolean): string {
    if (!isReady) {
        return "waiting";
    } else {
        return "ready";
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-negated-condition]: two-way branch uses a negated condition
 ──▶ main.tspp:2:9
  │
1 │ function status(isReady: boolean): string {
2 │     if (!isReady) {
  │         ^^^^^^^^
3 │         return "waiting";
4 │     } else {
  │
"#,
        );
    }

    /// Preserve branch comments by omitting the ternary fix.
    #[test]
    fn test_reports_commented_negated_ternary_without_fix() {
        let session = TestSession::dir(
            &NO_NEGATED_CONDITION,
            r#"
function status(isReady: boolean): string {
    return !isReady ? /* retain */ "waiting" : "ready";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-negated-condition]: two-way branch uses a negated condition
 ──▶ main.tspp:2:12
  │
1 │ function status(isReady: boolean): string {
2 │     return !isReady ? /* retain */ "waiting" : "ready";
  │            ^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
