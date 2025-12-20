use destack_ast::{self as ast, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `++` and `--` operators.
    ///
    /// The increment and decrement operators can lead to confusion with
    /// their prefix vs postfix semantics. Use `+= 1` or `-= 1` instead.
    #[lint(
        id = "no-plusplus",
        code = "LR010",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoPlusplus,
    "Disallow `++` and `--` operators"
}

impl LintRule for NoPlusplus {
    fn meta(&self) -> &'static crate::LintMeta {
        NoPlusplus::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::Unary { operator, .. } = expression else {
                continue;
            };
            if !matches!(
                operator,
                UnaryOperator::PreIncrement
                    | UnaryOperator::PostIncrement
                    | UnaryOperator::PreDecrement
                    | UnaryOperator::PostDecrement
            ) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_PLUSPLUS.id,
                    NO_PLUSPLUS.code,
                    NO_PLUSPLUS.category,
                    severity,
                    "`++` and `--` operators are not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use `+= 1` or `-= 1` instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_post_increment() {
        let test = TestProgram::for_rule_without_builtins(NoPlusplus);
        let result = test.lint_ast("test.ts", "x++;");
        test.result(result).assert_lint("no-plusplus");
    }

    #[test]
    fn test_detects_pre_increment() {
        let test = TestProgram::for_rule_without_builtins(NoPlusplus);
        let result = test.lint_ast("test.ts", "++x;");
        test.result(result).assert_lint("no-plusplus");
    }

    #[test]
    fn test_detects_post_decrement() {
        let test = TestProgram::for_rule_without_builtins(NoPlusplus);
        let result = test.lint_ast("test.ts", "x--;");
        test.result(result).assert_lint("no-plusplus");
    }

    #[test]
    fn test_detects_pre_decrement() {
        let test = TestProgram::for_rule_without_builtins(NoPlusplus);
        let result = test.lint_ast("test.ts", "--x;");
        test.result(result).assert_lint("no-plusplus");
    }

    #[test]
    fn test_allows_plus_equals() {
        let test = TestProgram::for_rule_without_builtins(NoPlusplus);
        let result = test.lint_ast("test.ts", "x += 1;");
        test.result(result).assert_no_lint("no-plusplus");
    }
}
