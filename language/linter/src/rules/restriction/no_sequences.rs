use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow sequence expressions (comma operator).
    ///
    /// The comma operator evaluates expressions left-to-right and returns the last value.
    /// This is confusing and error-prone. Use separate statements instead.
    /// Note: In `.ds` files, `(a, b, c)` is a tuple literal, not a sequence expression.
    #[lint(
        id = "no-sequences",
        code = "LR028",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoSequences,
    "Disallow sequence expressions"
}

impl LintRule for NoSequences {
    fn meta(&self) -> &'static crate::LintMeta {
        NoSequences::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(expression, ast::Expression::SequenceExpression { .. }) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_SEQUENCES.id,
                    NO_SEQUENCES.code,
                    NO_SEQUENCES.category,
                    severity,
                    "sequence expression is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use separate statements instead of comma operator"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_sequence_expression() {
        let test = TestProgram::for_rule_without_builtins(NoSequences);
        let result = test.lint_ast(
            "test.ts",
            r#"
let x = (1, 2, 3);
"#,
        );
        test.result(result).assert_lint("no-sequences");
    }

    #[test]
    fn test_allows_function_calls_with_multiple_args() {
        let test = TestProgram::for_rule_without_builtins(NoSequences);
        let result = test.lint_ast(
            "test.ts",
            r#"
foo(1, 2, 3);
"#,
        );
        test.result(result).assert_no_lint("no-sequences");
    }

    #[test]
    fn test_allows_array_literals() {
        let test = TestProgram::for_rule_without_builtins(NoSequences);
        let result = test.lint_ast(
            "test.ts",
            r#"
let arr = [1, 2, 3];
"#,
        );
        test.result(result).assert_no_lint("no-sequences");
    }

    #[test]
    fn test_allows_destack_tuples() {
        let test = TestProgram::for_rule_without_builtins(NoSequences);
        // in .ds files, (1, 2) is a tuple, not a sequence
        let result = test.lint_ast(
            "test.ds",
            r#"
let tuple = (1, 2, 3);
"#,
        );
        test.result(result).assert_no_lint("no-sequences");
    }
}
