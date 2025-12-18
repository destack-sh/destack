use destack_ast::{self as ast};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow magic numbers.
    ///
    /// Magic numbers are unnamed numeric literals that appear in code without
    /// explanation. Extract them into named constants for better readability.
    /// Common values like 0, 1, and -1 are allowed by default.
    #[lint(
        id = "no-magic-numbers",
        code = "LR013",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoMagicNumbers,
    "Disallow magic numbers"
}

impl LintRule for NoMagicNumbers {
    fn meta(&self) -> &'static crate::LintMeta {
        NoMagicNumbers::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let allowed_numbers = &ctx.options.allowed_magic_numbers;
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // extract numeric literals
            let expression = ctx.tree.get(node_id);
            let ast::Expression::ScalarLiteral(literal) = expression else {
                continue;
            };

            // check if the number is allowed
            let is_allowed = match literal {
                ast::ScalarLiteral::Integer(value) => allowed_numbers.contains(&(*value as f64)),
                ast::ScalarLiteral::Float(value) => allowed_numbers.contains(value),
                _ => true,
            };
            if is_allowed {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_MAGIC_NUMBERS.id,
                    NO_MAGIC_NUMBERS.code,
                    NO_MAGIC_NUMBERS.category,
                    severity,
                    "magic number detected",
                    ctx.module.file_id,
                    span,
                )
                .with_label("extract into a named constant"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_magic_integer() {
        let test = TestProgram::for_rule(NoMagicNumbers);
        let result = test.lint_ast("test.ts", "let x = 42;");
        test.result(result).assert_lint("no-magic-numbers");
    }

    #[test]
    fn test_detects_magic_float() {
        let test = TestProgram::for_rule(NoMagicNumbers);
        let result = test.lint_ast("test.ts", "let x = 3.14;");
        test.result(result).assert_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_zero() {
        let test = TestProgram::for_rule(NoMagicNumbers);
        let result = test.lint_ast("test.ts", "let x = 0;");
        test.result(result).assert_no_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_one() {
        let test = TestProgram::for_rule(NoMagicNumbers);
        let result = test.lint_ast("test.ts", "let x = 1;");
        test.result(result).assert_no_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_negative_one() {
        let test = TestProgram::for_rule(NoMagicNumbers);
        let result = test.lint_ast("test.ts", "let x = -1;");
        test.result(result).assert_no_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_two() {
        let test = TestProgram::for_rule(NoMagicNumbers);
        let result = test.lint_ast("test.ts", "let x = 2;");
        test.result(result).assert_no_lint("no-magic-numbers");
    }

    #[test]
    fn test_allows_strings() {
        let test = TestProgram::for_rule(NoMagicNumbers);
        let result = test.lint_ast("test.ts", r#"let x = "hello";"#);
        test.result(result).assert_no_lint("no-magic-numbers");
    }
}
