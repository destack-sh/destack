use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, PatchSet};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require NaN predicates for equality-based tests.
    pub USE_ISNAN {
        id: "use-isnan",
        summary: "Require NaN predicates for equality-based tests",
        explanation: r#"
Equality comparisons and Array index searches with NaN have fixed results because NaN is unequal to every value, including itself.
Instead, you MUST call `.isNaN()` in the direct test or search predicate.
"#,
        example: {
            reported: r#"
function isMissing(value: float64): boolean {
    return value === 0.0 / 0.0;
}
"#,
            accepted: r#"
function isMissing(value: float64): boolean {
    return value.isNaN();
}
"#,
        },
        provenance: [Eslint("use-isnan")],
        category: Correctness,
        level: Error,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One equality comparison with one or two exact NaN operands.
#[derive(Debug, Clone, Copy)]
enum NanEquality {
    /// One NaN operand compared with another value.
    Single {
        /// The NaN expression.
        nan: dir::LocalNodeId<dir::Expression>,
        /// The tested expression.
        value: dir::LocalNodeId<dir::Expression>,
        /// Whether the replacement negates the predicate.
        is_negated: bool,
    },
    /// Two NaN operands compared with each other.
    Both {
        /// The first NaN expression.
        nan: dir::LocalNodeId<dir::Expression>,
        /// Whether the comparison tests inequality.
        is_negated: bool,
    },
}

impl NanEquality {
    /// Select one NaN comparison.
    fn select(
        module: &DirModule<'_>,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
    ) -> Result<Option<Self>, ProviderError> {
        let left_is_nan = module.is_nan(left)?;
        let right_is_nan = module.is_nan(right)?;
        let comparison = match (left_is_nan, right_is_nan) {
            (true, false) => Self::Single {
                nan: left,
                value: right,
                is_negated: operator.is_negative_equality(),
            },
            (false, true) => Self::Single {
                nan: right,
                value: left,
                is_negated: operator.is_negative_equality(),
            },
            (true, true) => Self::Both {
                nan: left,
                is_negated: operator.is_negative_equality(),
            },
            (false, false) => return Ok(None),
        };

        Ok(Some(comparison))
    }

    /// Return the NaN operand highlighted by this comparison.
    fn nan(self) -> dir::LocalNodeId<dir::Expression> {
        match self {
            Self::Single { nan, .. } | Self::Both { nan, .. } => nan,
        }
    }

    /// Return the help text for this comparison.
    fn help(self) -> &'static str {
        match self {
            Self::Single {
                is_negated: true, ..
            } => "use a negated NaN predicate instead",
            Self::Single {
                is_negated: false, ..
            } => "use a NaN predicate instead",
            Self::Both {
                is_negated: true, ..
            } => "the comparison always evaluates to true",
            Self::Both {
                is_negated: false, ..
            } => "the comparison always evaluates to false",
        }
    }

    /// Build a review suggestion using the canonical NaN predicate.
    fn suggestion(
        self,
        module: &DirModule<'_>,
        comparison: dir::LocalNodeId<dir::Expression>,
        lint: &Lint,
    ) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
        let Self::Single {
            value, is_negated, ..
        } = self
        else {
            return Ok(None);
        };
        let comparison_span = module.source_extent(comparison.into_any())?;
        let value_span = module.source_extent(value.into_any())?;

        // do not discard comments outside the retained expression
        if module.has_unretained_comment(comparison_span, &[value_span])? {
            return Ok(None);
        }

        // preserve the exact compared expression text
        let receiver = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
        let predicate = format!("{receiver}.isNaN()");
        let replacement = if is_negated {
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

/// One Array index search for an exact NaN value.
#[derive(Debug, Clone, Copy)]
struct NanSearch {
    /// The complete callee expression.
    callee: dir::LocalNodeId<dir::Expression>,
    /// The NaN search argument.
    nan: dir::LocalNodeId<dir::Expression>,
    /// The predicate-based search method.
    method: &'static str,
    /// Whether the complete search can be replaced directly.
    is_replaceable: bool,
}

impl NanSearch {
    /// Select one canonical Array index search for NaN.
    fn select(
        module: &DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        let Some(call) = module.member_call(expression) else {
            return Ok(None);
        };

        let method = match module.language_member(expression)? {
            Some(member) if member == dir::LanguageItem::Array.member("indexOf") => "findIndex",
            Some(member) if member == dir::LanguageItem::Array.member("lastIndexOf") => {
                "findLastIndex"
            }
            _ => return Ok(None),
        };

        let Some(argument) = call.arguments.first() else {
            return Ok(None);
        };
        let dir::Argument::Positional { value: nan } = module.view().get(*argument) else {
            return Ok(None);
        };
        if !module.is_nan(*nan)? {
            return Ok(None);
        }

        // permit a rewrite only when NaN is the complete argument list
        let is_replaceable = call.generic_arguments.is_empty() && call.arguments.len() == 1;

        Ok(Some(Self {
            callee: call.callee,
            nan: *nan,
            method,
            is_replaceable,
        }))
    }

    /// Build the predicate-based Array search replacement.
    fn suggestion(
        self,
        module: &DirModule<'_>,
        lint: &Lint,
    ) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
        if !self.is_replaceable {
            return Ok(None);
        }

        let nan = module.source_extent(self.nan.into_any())?;
        if module.has_unretained_comment(nan, &[])? {
            return Ok(None);
        }

        // replace the method and its ineffective search value
        let mut file = FilePatch::new(nan.file);
        file.replace(module.main_span(self.callee.into_any())?, self.method);
        file.replace(nan, "(value) => value.isNaN()");
        file.sort();
        let suggestion = lint.suggestion("search with a NaN predicate", file)?;

        Ok(Some(suggestion))
    }
}

/// Report ineffective equality-based NaN tests.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect equality and switch expressions
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

                // select one NaN comparison
                let Some(comparison) = NanEquality::select(module, *left, *right, *operator)?
                else {
                    continue;
                };

                // report the ineffective equality comparison
                let span = module.source_extent(comparison.nan().into_any())?;
                let mut diagnostic = lint
                    .diagnostic("equality cannot test for NaN", span)
                    .help(comparison.help());
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

                // collect cases that use the builtin equality
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

    // inspect canonical Array index searches
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(search) = NanSearch::select(module, expression)? else {
            continue;
        };

        // report the ineffective equality-based search
        let span = module.source_extent(search.nan.into_any())?;
        let mut diagnostic = lint
            .diagnostic("Array index search cannot find NaN", span)
            .help("search with an `.isNaN()` predicate");
        if let Some(suggestion) = search.suggestion(module, lint)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report inequality with a NaN constant expression.
    #[test]
    fn test_reports_nan_inequality() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function isPresent(value: float64): boolean {
    return value !== 0.0 / 0.0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: equality cannot test for NaN
 ──▶ main.ds:2:22
  │
1 │ function isPresent(value: float64): boolean {
2 │     return value !== 0.0 / 0.0;
  │                      ^^^^^^^^^
3 │ }
  │

 = help: use a negated NaN predicate instead
 = suggestion: replace the equality check with a NaN predicate (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function isPresent(value: float64): boolean {
-   2│     return value !== 0.0 / 0.0;
+   2│     return !value.isNaN();
    3│ }
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
    3│ }
"#,
        );
    }

    /// Report equality with a signed NaN constant.
    #[test]
    fn test_reports_signed_nan_equality() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function isMissing(value: float64): boolean {
    return value === -NaN;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: equality cannot test for NaN
 ──▶ main.ds:2:22
  │
1 │ function isMissing(value: float64): boolean {
2 │     return value === -NaN;
  │                      ^^^^
3 │ }
  │

 = help: use a NaN predicate instead
 = suggestion: replace the equality check with a NaN predicate (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function isMissing(value: float64): boolean {
-   2│     return value === -NaN;
+   2│     return value.isNaN();
    3│ }
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
    3│ }
"#,
        );
    }

    /// Diagnose two NaN operands without changing the comparison result.
    #[test]
    fn test_reports_two_nan_operands_without_suggestion() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
const same = NaN === Number.NaN;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: equality cannot test for NaN
 ──▶ main.ds:1:14
  │
1 │ const same = NaN === Number.NaN;
  │              ^^^
  │

 = help: the comparison always evaluates to false
"#,
        );
    }

    /// Replace Array index searches with predicate-based searches.
    #[test]
    fn test_replaces_nan_index_searches() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function locate(values: float64[]): isize | undefined {
    return values.indexOf(NaN);
}

function locateLast(values: float64[] | undefined): isize | undefined {
    return values?.lastIndexOf(Number.NaN);
}
"#,
        );

        session.assert_suggestions(
            r#"
function locate(values: float64[]): isize | undefined {
    return values.findIndex((value) => value.isNaN());
}

function locateLast(values: float64[] | undefined): isize | undefined {
    return values?.findLastIndex((value) => value.isNaN());
}
"#,
        );
    }

    /// Diagnose a NaN index search whose starting position prevents a direct replacement.
    #[test]
    fn test_reports_nan_index_search_with_start() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
function locate(values: float64[], start: isize): isize | undefined {
    return values.indexOf(NaN, start);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[use-isnan]: Array index search cannot find NaN
 ──▶ main.ds:2:27
  │
1 │ function locate(values: float64[], start: isize): isize | undefined {
2 │     return values.indexOf(NaN, start);
  │                           ^^^
3 │ }
  │

 = help: search with an `.isNaN()` predicate
"#,
        );
    }

    /// Accept a user-defined index search with custom equality behavior.
    #[test]
    fn test_accepts_user_index_search() {
        let session = TestSession::dir(
            &USE_ISNAN,
            r#"
class Values {
    indexOf(value: float64): isize | undefined {
        return 0;
    }
}

declare const values: Values;
const index = values.indexOf(NaN);
"#,
        );

        session.assert_no_diagnostics();
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
    4│ }
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
    equal(&readonly this, other: &readonly float64): boolean {
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
