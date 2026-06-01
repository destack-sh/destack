use destack_dir::{self as dir};
use destack_workspace::LintSeverity;

use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow magic numbers.
    ///
    /// Magic numbers are unnamed numeric literals that appear in code without
    /// explanation. Extract them into named constants for better readability.
    /// Common values like 0, 1, and -1 are allowed by default.
    #[lint(
        id = "no-magic-numbers",
        code = "LR016",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoMagicNumbers,
    "Disallow magic numbers"
}

impl LintRule for NoMagicNumbers {
    fn meta(&self) -> &'static LintMeta {
        NoMagicNumbers::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let allowed_numbers = ctx.options().restriction.allowed_magic_numbers.clone();

        // inspect candidate expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            // extract numeric literals with unary sign context
            let expression = ctx.dir.get(node_id);
            let dir::Expression::ScalarLiteral(literal) = expression else {
                continue;
            };
            let Some((numeric_value, report_expression_id)) =
                numeric_literal_value_and_report_expression(ctx, node_id, literal)
            else {
                continue;
            };
            if allowed_numbers.contains(&numeric_value) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, report_expression_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.dir.get_span(report_expression_id);
            ctx.report(
                LintReport::new(
                    NO_MAGIC_NUMBERS.id,
                    NO_MAGIC_NUMBERS.code,
                    NO_MAGIC_NUMBERS.category,
                    severity,
                    "magic number detected",
                    span,
                )
                .label("extract into a named constant"),
            );
        }
    }
}

/// Resolve one numeric literal value and the expression span that should be reported.
fn numeric_literal_value_and_report_expression(
    ctx: &LintModuleContext<'_>,
    literal_expression_id: dir::LocalNodeId<dir::Expression>,
    literal: &dir::ScalarLiteral,
) -> Option<(f64, dir::LocalNodeId<dir::Expression>)> {
    let literal_value = match literal {
        dir::ScalarLiteral::Integer(value) => *value as f64,
        dir::ScalarLiteral::Float(value) => *value,
        _ => return None,
    };

    // require optional structure
    let Some(parent_id) = ctx.dir.get_parent_id(literal_expression_id.id) else {
        return Some((literal_value, literal_expression_id));
    };
    if ctx.dir.get_node_type(parent_id) != dir::NodeType::Expression {
        return Some((literal_value, literal_expression_id));
    }

    // resolve parent expression id
    let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
    let parent_expression = ctx.dir.get(parent_expression_id);
    let dir::Expression::Unary { operator, right } = parent_expression else {
        return Some((literal_value, literal_expression_id));
    };
    if *right != literal_expression_id {
        return Some((literal_value, literal_expression_id));
    }

    // branch by expression kind
    match operator {
        dir::UnaryOperator::Negate => Some((-literal_value, parent_expression_id)),
        dir::UnaryOperator::Plus => Some((literal_value, parent_expression_id)),
        _ => Some((literal_value, literal_expression_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_magic_integer() {
        let test = TestProgram::for_rule_without_prelude(NoMagicNumbers);
        let result = test.lint(
            "no_magic_numbers/test_detects_magic_integer.ts",
            "let x = 42;",
        );
        test.result(result).assert_lint("no-magic-numbers");
    }

    #[test]
    fn test_detects_magic_float() {
        let test = TestProgram::for_rule_without_prelude(NoMagicNumbers);
        let result = test.lint(
            "no_magic_numbers/test_detects_magic_float.ts",
            "let x = 3.14;",
        );
        test.result(result).assert_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_zero() {
        let test = TestProgram::for_rule_without_prelude(NoMagicNumbers);
        let result = test.lint("no_magic_numbers/test_allows_zero.ts", "let x = 0;");
        test.result(result).assert_no_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_one() {
        let test = TestProgram::for_rule_without_prelude(NoMagicNumbers);
        let result = test.lint("no_magic_numbers/test_allows_one.ts", "let x = 1;");
        test.result(result).assert_no_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_negative_one() {
        let test = TestProgram::for_rule_without_prelude(NoMagicNumbers);
        let result = test.lint(
            "no_magic_numbers/test_allows_negative_one.ts",
            "let x = -1;",
        );
        test.result(result).assert_no_lint("no-magic-numbers");
    }

    #[test]
    fn test_detects_negative_magic_number() {
        let test = TestProgram::for_rule_without_prelude(NoMagicNumbers);
        let result = test.lint(
            "no_magic_numbers/test_detects_negative_magic_number.ts",
            "let x = -3;",
        );
        test.result(result).assert_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_configured_negative_magic_number() {
        let test = TestProgram::for_rule_without_prelude(NoMagicNumbers).with_options(|options| {
            options.restriction.allowed_magic_numbers.push(-3.0);
        });
        let result = test.lint(
            "no_magic_numbers/test_allows_configured_negative_magic_number.ts",
            "let x = -3;",
        );
        test.result(result).assert_no_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_two() {
        let test = TestProgram::for_rule_without_prelude(NoMagicNumbers);
        let result = test.lint("no_magic_numbers/test_allows_two.ts", "let x = 2;");
        test.result(result).assert_no_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_strings() {
        let test = TestProgram::for_rule_without_prelude(NoMagicNumbers);
        let result = test.lint(
            "no_magic_numbers/test_allows_strings.ts",
            r#"let x = "hello";"#,
        );
        test.result(result).assert_no_lint("no-magic-numbers");
    }
}
