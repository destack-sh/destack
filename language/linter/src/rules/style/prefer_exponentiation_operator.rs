use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer the exponentiation operator to a well-known power function.
    pub PREFER_EXPONENTIATION_OPERATOR {
        id: "prefer-exponentiation-operator",
        summary: "Prefer the exponentiation operator to a well-known power function",
        explanation: r#"
The canonical `Math.pow(base, exponent)` call performs the same operation as `base ** exponent`.
Instead, you SHOULD use the exponentiation operator.
"#,
        example: {
            reported: r#"
function square(value: number): number {
    return Math.pow(value, 2);
}
"#,
            accepted: r#"
function square(value: number): number {
    return value ** 2;
}
"#,
        },
        provenance: [Eslint("prefer-exponentiation-operator")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical power calls written without the exponentiation operator.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Math.pow calls with two positional operands
    for expression in module.call_expressions() {
        let expression = expression?;
        let node = view.get(expression);
        let dir::Expression::Call { arguments, .. } = node else {
            continue;
        };

        // require the canonical power function
        if module.language_member(expression)? != Some(dir::LanguageItem::Math.member("pow")) {
            continue;
        }

        // require two positional operands
        let [base, exponent] = arguments.as_slice() else {
            continue;
        };
        let (
            dir::Argument::Positional { value: base },
            dir::Argument::Positional { value: exponent },
        ) = (view.get(*base), view.get(*exponent))
        else {
            continue;
        };

        // replace the call while preserving operator grouping
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("Math.pow call obscures exponentiation", span);
        if let Some(suggestion) = suggestion(module, lint, expression, *base, *exponent)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one precedence-safe exponentiation expression.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    base: dir::LocalNodeId<dir::Expression>,
    exponent: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let base_span = module.source_extent(base.into_any())?;
    let exponent_span = module.source_extent(exponent.into_any())?;
    if module.has_unretained_comment(extent, &[base_span, exponent_span])? {
        return Ok(None);
    }

    // group both operands according to exponentiation precedence
    let base_source = module.expression_source(base, dir::OperatorPrecedence::Postfix)?;
    let exponent_source =
        module.expression_source(exponent, dir::OperatorPrecedence::Exponentiation)?;

    let replacement = format!("{base_source} ** {exponent_source}");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use the exponentiation operator", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Parenthesize operands according to exponentiation precedence.
    #[test]
    fn test_groups_exponentiation_operands() {
        let session = TestSession::dir(
            &PREFER_EXPONENTIATION_OPERATOR,
            r#"
function power(left: number, right: number, exponent: number): number {
    return Math.pow(left + right, exponent + 1);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-exponentiation-operator]: Math.pow call obscures exponentiation
 ──▶ main.tspp:2:12
  │
1 │ function power(left: number, right: number, exponent: number): number {
2 │     return Math.pow(left + right, exponent + 1);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the exponentiation operator
--- a/main.tspp
+++ b/main.tspp

    1│ function power(left: number, right: number, exponent: number): number {
-   2│     return Math.pow(left + right, exponent + 1);
+   2│     return (left + right) ** (exponent + 1);
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function power(left: number, right: number, exponent: number): number {
    return (left + right) ** (exponent + 1);
}
"#,
        );
    }

    /// Parenthesize a unary base, which cannot directly precede exponentiation.
    #[test]
    fn test_groups_unary_base() {
        let session = TestSession::dir(
            &PREFER_EXPONENTIATION_OPERATOR,
            r#"
function power(value: number, exponent: number): number {
    return Math.pow(-value, exponent);
}
"#,
        );

        session.assert_fixes(
            r#"
function power(value: number, exponent: number): number {
    return (-value) ** exponent;
}
"#,
        );
    }

    /// Accept a user-defined method named `pow`.
    #[test]
    fn test_accepts_user_defined_pow() {
        let session = TestSession::dir(
            &PREFER_EXPONENTIATION_OPERATOR,
            r#"
class Calculator {
    pow(base: number, exponent: number): number {
        return base ** exponent;
    }
}
function power(calculator: Calculator): number {
    return calculator.pow(2, 8);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comments between power operands by omitting the fix.
    #[test]
    fn test_reports_commented_power_without_fix() {
        let session = TestSession::dir(
            &PREFER_EXPONENTIATION_OPERATOR,
            r#"
function power(value: number): number {
    return Math.pow(value /* retain */, 2);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-exponentiation-operator]: Math.pow call obscures exponentiation
 ──▶ main.tspp:2:12
  │
1 │ function power(value: number): number {
2 │     return Math.pow(value /* retain */, 2);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
