use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty destructuring patterns.
    ///
    /// Empty destructuring patterns like `const {} = obj` or `const [] = arr`
    /// don't bind any values and are likely mistakes.
    #[lint(
        id = "no-empty-pattern",
        code = "LU012",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoEmptyPattern,
    "Disallow empty destructuring patterns"
}

impl LintRule for NoEmptyPattern {
    fn meta(&self) -> &'static crate::LintMeta {
        NoEmptyPattern::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Pattern>() {
            let pattern = ctx.tree.get(node_id);

            let is_empty = match pattern {
                ast::Pattern::Object { fields } => fields.is_empty(),
                ast::Pattern::Array { fields } => fields.is_empty(),
                ast::Pattern::Tuple { fields } => fields.is_empty(),
                _ => false,
            };

            if is_empty {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let kind = match pattern {
                    ast::Pattern::Object { .. } => "object",
                    ast::Pattern::Array { .. } => "array",
                    ast::Pattern::Tuple { .. } => "tuple",
                    _ => unreachable!(),
                };

                ctx.report(
                    LintDiagnostic::new(
                        NO_EMPTY_PATTERN.id,
                        NO_EMPTY_PATTERN.code,
                        NO_EMPTY_PATTERN.category,
                        severity,
                        format!("empty {kind} destructuring pattern"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("this pattern doesn't bind any values"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_object_pattern() {
        let test = TestProgram::for_rule(NoEmptyPattern);
        let result = test.lint_ast(
            "test.ds",
            r#"
const {} = obj
"#,
        );
        test.result(result).assert_lint("no-empty-pattern");
    }

    #[test]
    fn test_detects_empty_array_pattern() {
        let test = TestProgram::for_rule(NoEmptyPattern);
        let result = test.lint_ast(
            "test.ds",
            r#"
const [] = arr
"#,
        );
        test.result(result).assert_lint("no-empty-pattern");
    }

    #[test]
    fn test_detects_empty_tuple_pattern() {
        let test = TestProgram::for_rule(NoEmptyPattern);
        let result = test.lint_ast(
            "test.ds",
            r#"
const () = tuple
"#,
        );
        test.result(result).assert_lint("no-empty-pattern");
    }

    #[test]
    fn test_detects_empty_pattern_in_function_param() {
        let test = TestProgram::for_rule(NoEmptyPattern);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo({}) {}
"#,
        );
        test.result(result).assert_lint("no-empty-pattern");
    }

    #[test]
    fn test_allows_non_empty_object_pattern() {
        let test = TestProgram::for_rule(NoEmptyPattern);
        let result = test.lint_ast(
            "test.ds",
            r#"
const { x } = obj
"#,
        );
        test.result(result).assert_no_lint("no-empty-pattern");
    }

    #[test]
    fn test_allows_non_empty_array_pattern() {
        let test = TestProgram::for_rule(NoEmptyPattern);
        let result = test.lint_ast(
            "test.ds",
            r#"
const [x] = arr
"#,
        );
        test.result(result).assert_no_lint("no-empty-pattern");
    }
}
