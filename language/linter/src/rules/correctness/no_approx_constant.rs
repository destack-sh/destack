use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow approximate representations of mathematical constants.
    ///
    /// Using approximate values like `3.14` instead of `Math.PI` can lead to
    /// precision issues and makes the intent less clear. Use the standard
    /// library constants instead.
    #[lint(
        id = "no-approx-constant",
        code = "LC016",
        category = Correctness,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoApproxConstant,
    "Disallow approximate math constants"
}

// mathematical constants: (name, value, minimum prefix to match)
// we match if the literal starts with the prefix and is <= the constant
const CONSTANTS: &[(&str, f64, &str)] = &[
    ("PI", std::f64::consts::PI, "3.14"),
    ("E", std::f64::consts::E, "2.71"),
    ("SQRT_2", std::f64::consts::SQRT_2, "1.41"),
    ("LN_2", std::f64::consts::LN_2, "0.69"),
    ("LN_10", std::f64::consts::LN_10, "2.30"),
    ("TAU", std::f64::consts::TAU, "6.28"),
];

impl LintRule for NoApproxConstant {
    fn meta(&self) -> &'static crate::LintMeta {
        NoApproxConstant::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let Expression::ScalarLiteral(ScalarLiteral::Float(value)) = expression else {
                continue;
            };

            // check if this float approximates any known constant
            if let Some((name, _constant)) = find_approximate_constant(*value) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_APPROX_CONSTANT.id,
                        NO_APPROX_CONSTANT.code,
                        NO_APPROX_CONSTANT.category,
                        severity,
                        format!("approximate value of `Math.{name}`"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label(format!("use `Math.{name}` instead")),
                );
            }
        }
    }
}

/// find if a float value approximates a known mathematical constant
fn find_approximate_constant(value: f64) -> Option<(&'static str, f64)> {
    // format the value as a string for prefix matching
    let value_str = format!("{value}");
    for &(name, constant, prefix) in CONSTANTS {
        // check if the value string starts with the constant's prefix
        if value_str.starts_with(prefix) {
            // also verify the value is <= the constant (it's a truncation)
            if value <= constant {
                return Some((name, constant));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_approx_pi() {
        let test = TestProgram::for_rule(NoApproxConstant);
        let result = test.lint_ast(
            "test.ds",
            r#"
let pi = 3.14
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }

    #[test]
    fn test_detects_approx_pi_more_digits() {
        let test = TestProgram::for_rule(NoApproxConstant);
        let result = test.lint_ast(
            "test.ds",
            r#"
let pi = 3.14159
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }

    #[test]
    fn test_detects_approx_e() {
        let test = TestProgram::for_rule(NoApproxConstant);
        let result = test.lint_ast(
            "test.ds",
            r#"
let e = 2.71828
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }

    #[test]
    fn test_detects_approx_sqrt2() {
        let test = TestProgram::for_rule(NoApproxConstant);
        let result = test.lint_ast(
            "test.ds",
            r#"
let sqrt2 = 1.414
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }

    #[test]
    fn test_allows_unrelated_floats() {
        let test = TestProgram::for_rule(NoApproxConstant);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1.5
let y = 2.0
let z = 0.5
"#,
        );
        test.result(result).assert_no_lint("no-approx-constant");
    }

    #[test]
    fn test_allows_small_integers_as_floats() {
        let test = TestProgram::for_rule(NoApproxConstant);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 3.0
let y = 2.0
"#,
        );
        test.result(result).assert_no_lint("no-approx-constant");
    }
}
