use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer one comparison over an equivalent pair of comparisons.
    pub DOUBLE_COMPARISONS {
        id: "double-comparisons",
        summary: "Prefer one comparison over an equivalent pair of comparisons",
        explanation: r#"
Two comparisons that repeat identical operand computations can encode the same relation as one comparison.
Instead, you SHOULD use the equivalent single operator.
"#,
        example: {
            reported: r#"
function atMost(left: int32, right: int32): boolean {
    return left < right || left === right;
}
"#,
            accepted: r#"
function atMost(left: int32, right: int32): boolean {
    return left <= right;
}
"#,
        },
        provenance: [Clippy("double_comparisons")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report equivalent pairs of builtin comparisons.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect conjunctions and disjunctions of two binary comparisons
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((logical, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        if !matches!(logical, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
            continue;
        }

        // require builtin comparisons on both sides
        let Some((left_operator, [left_first, left_second])) =
            module.builtin_binary(left.source.local_id)?
        else {
            continue;
        };
        if !left_operator.is_comparison() {
            continue;
        }
        let Some((right_operator, [right_first, right_second])) =
            module.builtin_binary(right.source.local_id)?
        else {
            continue;
        };
        if !right_operator.is_comparison() {
            continue;
        }

        // align the second comparison with the first comparison's operand order
        let is_aligned = module.is_same_operand(left_first, right_first)?
            && module.is_same_operand(left_second, right_second)?;
        let is_swapped = module.is_same_operand(left_first, right_second)?
            && module.is_same_operand(left_second, right_first)?;
        if !is_aligned && !is_swapped {
            continue;
        }

        // normalize the second comparison to the first operand order
        let right_operator = if is_aligned {
            right_operator
        } else {
            let Some(operator) = right_operator.swapped() else {
                continue;
            };

            operator
        };

        // select the equivalent single operator
        let Some(operator) = combined_operator(logical, left_operator, right_operator) else {
            continue;
        };

        // replace the pair while retaining the first comparison's operands
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("two comparisons express one relationship", span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            expression,
            left_first.source.local_id,
            left_second.source.local_id,
            operator,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the single comparison equivalent to one logical pair.
fn combined_operator(
    logical: dir::BinaryOperator,
    left: dir::BinaryOperator,
    right: dir::BinaryOperator,
) -> Option<dir::BinaryOperator> {
    use dir::BinaryOperator::{
        And, Equal, EqualStrict, GreaterThan, GreaterThanOrEqual, LessThan, LessThanOrEqual,
        NotEqual, NotEqualStrict, Or,
    };

    match (logical, left, right) {
        (Or, LessThan, Equal | EqualStrict) | (Or, Equal | EqualStrict, LessThan) => {
            Some(LessThanOrEqual)
        }
        (Or, GreaterThan, Equal | EqualStrict) | (Or, Equal | EqualStrict, GreaterThan) => {
            Some(GreaterThanOrEqual)
        }
        (And, LessThanOrEqual, NotEqual | NotEqualStrict)
        | (And, NotEqual | NotEqualStrict, LessThanOrEqual) => Some(LessThan),
        (And, GreaterThanOrEqual, NotEqual | NotEqualStrict)
        | (And, NotEqual | NotEqualStrict, GreaterThanOrEqual) => Some(GreaterThan),
        _ => None,
    }
}

/// Build one comparison from the retained operands.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    left: dir::LocalNodeId<dir::Expression>,
    right: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let left_span = module.source_extent(left.into_any())?;
    let right_span = module.source_extent(right.into_any())?;
    if module.has_unretained_comment(extent, &[left_span, right_span])? {
        return Ok(None);
    }

    // retain the first comparison's operands with their required grouping
    let left = module.expression_source(left, dir::OperatorPrecedence::Comparison)?;
    let right = module.expression_source(right, dir::OperatorPrecedence::Comparison)?;
    let replacement = format!("{left} {} {right}", operator.text());
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use one comparison", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an inclusive conjunction written with swapped operands.
    #[test]
    fn test_replaces_swapped_inclusive_conjunction() {
        let session = TestSession::dir(
            &DOUBLE_COMPARISONS,
            r#"
function below(left: int32, right: int32): boolean {
    return left !== right && right >= left;
}
"#,
        );

        session.assert_fixes(
            r#"
function below(left: int32, right: int32): boolean {
    return left < right;
}
"#,
        );
    }

    /// Replace a greater-than disjunction with one inclusive comparison.
    #[test]
    fn test_replaces_greater_than_disjunction() {
        let session = TestSession::dir(
            &DOUBLE_COMPARISONS,
            r#"
function atLeast(left: int32, right: int32): boolean {
    return left > right || left === right;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[double-comparisons]: two comparisons express one relationship
 ──▶ main.tspp:2:12
  │
1 │ function atLeast(left: int32, right: int32): boolean {
2 │     return left > right || left === right;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use one comparison
--- a/main.tspp
+++ b/main.tspp

    1│ function atLeast(left: int32, right: int32): boolean {
-   2│     return left > right || left === right;
+   2│     return left >= right;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function atLeast(left: int32, right: int32): boolean {
    return left >= right;
}
"#,
        );
    }

    /// Replace an inclusive conjunction with one strict comparison.
    #[test]
    fn test_replaces_greater_than_conjunction() {
        let session = TestSession::dir(
            &DOUBLE_COMPARISONS,
            r#"
function above(left: int32, right: int32): boolean {
    return left >= right && left !== right;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[double-comparisons]: two comparisons express one relationship
 ──▶ main.tspp:2:12
  │
1 │ function above(left: int32, right: int32): boolean {
2 │     return left >= right && left !== right;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use one comparison
--- a/main.tspp
+++ b/main.tspp

    1│ function above(left: int32, right: int32): boolean {
-   2│     return left >= right && left !== right;
+   2│     return left > right;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function above(left: int32, right: int32): boolean {
    return left > right;
}
"#,
        );
    }

    /// Report without a fix when combining comparisons would discard a comment.
    #[test]
    fn test_reports_commented_comparisons_without_fix() {
        let session = TestSession::dir(
            &DOUBLE_COMPARISONS,
            r#"
function atMost(left: int32, right: int32): boolean {
    return left < right || /* retain */ left === right;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[double-comparisons]: two comparisons express one relationship
 ──▶ main.tspp:2:12
  │
1 │ function atMost(left: int32, right: int32): boolean {
2 │     return left < right || /* retain */ left === right;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Accept comparisons that repeat effectful calls.
    #[test]
    fn test_accepts_effectful_operands() {
        let session = TestSession::dir(
            &DOUBLE_COMPARISONS,
            r#"
declare function next(): int32;
function below(limit: int32): boolean {
    return next() < limit || next() === limit;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
