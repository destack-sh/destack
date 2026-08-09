use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, PatchSet};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require the NaN predicate instead of equality comparisons with NaN.
    pub USE_ISNAN {
        id: "use-isnan",
        summary: "Require the NaN predicate instead of equality comparisons with NaN",
        explanation: "Floating-point NaN is unequal to every value, including itself, so equality cannot test for it. Call `.isNaN()` on the checked float value to state the operation directly.",
        example: {
            reported: r#"
function isMissing(value: float64): boolean {
    return value === (0.0 / 0.0);
}
"#,
            accepted: r#"
function isMissing(value: float64): boolean {
    return value.isNaN();
}
"#,
        },
        category: Correctness,
        level: Error,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report equality comparisons whose checked operand is NaN.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect checked equality and switch expressions
    for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
        match expression {
            // value === NaN
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                if !operator.is_equality() {
                    continue;
                }
                let Some(resolution) = module.operator_decision(expression_id.into_any())? else {
                    continue;
                };
                if !resolution.is_builtin() {
                    continue;
                }

                // select one checked NaN comparison
                let Some(comparison) = NanEquality::select(module, *left, *right, *operator)?
                else {
                    continue;
                };

                // report the ineffective equality comparison
                let span = module.source_extent(comparison.nan.into_any())?;
                let predicate = if operator.is_negative_equality() {
                    "use a negated NaN predicate instead"
                } else {
                    "use a NaN predicate instead"
                };
                let mut diagnostic = lint
                    .diagnostic("equality cannot test for NaN", span)
                    .help(predicate);
                if let Some(suggestion) = comparison.suggestion(module, expression_id, lint)? {
                    diagnostic = diagnostic.suggestion(suggestion);
                }
                output.report(diagnostic);
            }

            // switch (NaN)
            dir::Expression::Switch { value, cases } => {
                let mut selectors = Vec::new();
                let mut has_selector = false;
                let mut uses_builtin_equality = true;

                // collect cases that use the checked builtin equality
                for case in cases {
                    let dir::SwitchSelector::Case(selector) = view.get(*case).selector else {
                        continue;
                    };
                    has_selector = true;
                    let is_builtin = module
                        .operator_decision(case.into_any())?
                        .is_some_and(dir::OperatorDecision::is_builtin);
                    uses_builtin_equality &= is_builtin;
                    if is_builtin {
                        selectors.push(selector);
                    }
                }

                // switch (NaN)
                if has_selector && uses_builtin_equality && module.is_nan(*value)? {
                    let span = module.source_extent(value.into_any())?;
                    let diagnostic = lint
                        .diagnostic("NaN switch value cannot match a case", span)
                        .help("use a NaN predicate before the switch");
                    output.report(diagnostic);
                }

                // case NaN
                for selector in selectors {
                    if !module.is_nan(selector)? {
                        continue;
                    }

                    let span = module.source_extent(selector.into_any())?;
                    let diagnostic = lint
                        .diagnostic("switch case cannot match NaN", span)
                        .help("test for NaN before entering the switch");
                    output.report(diagnostic);
                }
            }

            // unrelated expressions
            _ => {}
        }
    }

    Ok(output)
}

/// One equality comparison with an exact NaN operand.
#[derive(Debug, Clone, Copy)]
struct NanEquality {
    /// The NaN expression.
    nan: dir::LocalNodeId<dir::Expression>,
    /// The compared expression.
    value: dir::LocalNodeId<dir::Expression>,
    /// Whether the replacement negates the predicate.
    is_negated: bool,
}

impl NanEquality {
    /// Select one checked NaN comparison.
    fn select(
        module: &DirModule<'_>,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
    ) -> Result<Option<Self>, ProviderError> {
        let (nan, value) = if module.is_nan(left)? {
            (left, right)
        } else if module.is_nan(right)? {
            (right, left)
        } else {
            return Ok(None);
        };

        Ok(Some(Self {
            nan,
            value,
            is_negated: operator.is_negative_equality(),
        }))
    }

    /// Build a review suggestion using the canonical NaN predicate.
    fn suggestion(
        self,
        module: &DirModule<'_>,
        comparison: dir::LocalNodeId<dir::Expression>,
        lint: &Lint,
    ) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
        let comparison_span = module.source_extent(comparison.into_any())?;
        let value_span = module.source_extent(self.value.into_any())?;

        // do not discard comments outside the retained expression
        if module.has_unretained_comment(comparison_span, &[value_span])? {
            return Ok(None);
        }

        // preserve the exact compared expression text
        let receiver = module.postfix_source(self.value)?;
        let predicate = format!("{receiver}.isNaN()");
        let replacement = if self.is_negated {
            format!("!{predicate}")
        } else {
            predicate
        };

        // replace the complete ineffective comparison
        let mut file_patch = FilePatch::new(comparison_span.file);
        file_patch.replace(comparison_span, replacement);
        let patches = PatchSet::from_files(vec![file_patch]);

        let suggestion =
            lint.suggestion("replace the equality check with a NaN predicate", patches)?;

        Ok(Some(suggestion))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report inequality with a checked NaN constant expression.
    #[test]
    fn test_reports_nan_inequality() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function isPresent(value: float64): boolean {
    return value !== (0.0 / 0.0);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: equality cannot test for NaN
 ──▶ main.ds:2:22
  │
1 │ function isPresent(value: float64): boolean {
2 │     return value !== (0.0 / 0.0);
  │                      ^^^^^^^^^^^
3 │ }
  │

 = help: use a negated NaN predicate instead
 = suggestion: replace the equality check with a NaN predicate (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function isPresent(value: float64): boolean {
-   2│     return value !== (0.0 / 0.0);
+   2│     return !value.isNaN();
"#,
        );

        session.assert_suggestions(
            r#"
function isPresent(value: float64): boolean {
    return !value.isNaN();
}
"#,
        );
    }

    /// Report equality with the canonical NaN constant.
    #[test]
    fn test_reports_named_nan_equality() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function isMissing(value: float64): boolean {
    return value === NaN;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: equality cannot test for NaN
 ──▶ main.ds:2:22
  │
1 │ function isMissing(value: float64): boolean {
2 │     return value === NaN;
  │                      ^^^
3 │ }
  │

 = help: use a NaN predicate instead
 = suggestion: replace the equality check with a NaN predicate (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function isMissing(value: float64): boolean {
-   2│     return value === NaN;
+   2│     return value.isNaN();
"#,
        );
    }

    /// Report equality with the Number NaN constant.
    #[test]
    fn test_reports_number_nan_equality() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function isMissing(value: float64): boolean {
    return Number.NaN === value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: equality cannot test for NaN
 ──▶ main.ds:2:12
  │
1 │ function isMissing(value: float64): boolean {
2 │     return Number.NaN === value;
  │            ^^^^^^^^^^
3 │ }
  │

 = help: use a NaN predicate instead
 = suggestion: replace the equality check with a NaN predicate (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function isMissing(value: float64): boolean {
-   2│     return Number.NaN === value;
+   2│     return value.isNaN();
"#,
        );
    }

    /// Report equality with a user scalar constant that evaluates to NaN.
    #[test]
    fn test_reports_nan_constant_equality() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
const Missing = 0.0 / 0.0;
function isMissing(value: float64): boolean {
    return value === Missing;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: equality cannot test for NaN
 ──▶ main.ds:3:22
  │
1 │ const Missing = 0.0 / 0.0;
2 │ function isMissing(value: float64): boolean {
3 │     return value === Missing;
  │                      ^^^^^^^
4 │ }
  │

 = help: use a NaN predicate instead
 = suggestion: replace the equality check with a NaN predicate (requires review)
--- a/main.ds
+++ b/main.ds

    2│ function isMissing(value: float64): boolean {
-   3│     return value === Missing;
+   3│     return value.isNaN();
"#,
        );
    }

    /// Preserve comparison comments by omitting the review suggestion.
    #[test]
    fn test_reports_commented_nan_equality_without_suggestion() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function isMissing(value: float64): boolean {
    return value /* comparison */ === NaN;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: equality cannot test for NaN
 ──▶ main.ds:2:39
  │
1 │ function isMissing(value: float64): boolean {
2 │     return value /* comparison */ === NaN;
  │                                       ^^^
3 │ }
  │

 = help: use a NaN predicate instead
"#,
        );
    }

    /// Parenthesize a compound receiver in the review suggestion.
    #[test]
    fn test_parenthesizes_nan_predicate_receiver() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function isMissing(value: float64): boolean {
    return value + 1.0 === NaN;
}
"#,
        );

        session.assert_suggestions(
            r#"
function isMissing(value: float64): boolean {
    return (value + 1.0).isNaN();
}
"#,
        );
    }

    /// Accept the typed predicate through a generic Float bound.
    #[test]
    fn test_accepts_generic_float_predicate() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
import { Float } from "destack:math";

function isMissing<T: Float>(value: T): boolean {
    return value.isNaN();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept ordinary floating-point equality for this rule.
    #[test]
    fn test_accepts_finite_float_equality() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function isZero(value: float64): boolean {
    return value === 0.0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept NaN passed to user-defined equality.
    #[test]
    fn test_accepts_overloaded_nan_equality() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
import { PartialEqual } from "destack:ops";

struct Measure {
    value: float64;
}

extension of Measure implements PartialEqual<float64> {
    equal(other: float64): boolean {
        return this.value == other;
    }
}

declare const measure: Measure;
const same = measure == NaN;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a NaN switch case.
    #[test]
    fn test_reports_nan_switch_case() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function select(value: float64): void {
    switch (value) {
        case NaN:
            break;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: switch case cannot match NaN
 ──▶ main.ds:3:14
  │
1 │ function select(value: float64): void {
2 │     switch (value) {
3 │         case NaN:
  │              ^^^
4 │             break;
5 │     }
  │

 = help: test for NaN before entering the switch
"#,
        );
    }

    /// Report a NaN switch value.
    #[test]
    fn test_reports_nan_switch_value() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function select(): void {
    switch (NaN) {
        case 1.0:
            break;
        default:
            break;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: NaN switch value cannot match a case
 ──▶ main.ds:2:13
  │
1 │ function select(): void {
2 │     switch (NaN) {
  │             ^^^
3 │         case 1.0:
4 │             break;
  │

 = help: use a NaN predicate before the switch
"#,
        );
    }

    /// Accept a NaN switch value when no case performs equality.
    #[test]
    fn test_accepts_nan_switch_value_without_case() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function select(): void {
    switch (NaN) {
        default:
            break;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
