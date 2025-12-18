use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty functions.
    ///
    /// Empty functions are often a sign of incomplete code. If intentional,
    /// add a comment explaining why the function is empty.
    #[lint(
        id = "no-empty-function",
        code = "LU013",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoEmptyFunction,
    "Disallow empty functions"
}

impl LintRule for NoEmptyFunction {
    fn meta(&self) -> &'static crate::LintMeta {
        NoEmptyFunction::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let ast::Declaration::Function {
                body: Some(body_id),
                ..
            } = declaration
            else {
                continue;
            };

            // check if body is an empty block
            let body = ctx.tree.get(*body_id);
            let is_empty = match body {
                ast::Expression::Block(block_id) => {
                    let block = ctx.tree.get(*block_id);
                    block.expressions.is_empty() && !ctx.tree.has_infix_annotations(block_id.id)
                }
                _ => false,
            };

            if is_empty {
                ctx.report(
                    LintDiagnostic::new(
                        NO_EMPTY_FUNCTION.id,
                        NO_EMPTY_FUNCTION.code,
                        NO_EMPTY_FUNCTION.category,
                        severity,
                        "empty function",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("add implementation or a comment explaining why empty"),
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
    fn test_detects_empty_function() {
        let test = TestProgram::for_rule(NoEmptyFunction);
        let result = test.lint_ast("test.ds", "function foo() {}");
        test.result(result).assert_lint("no-empty-function");
    }

    #[test]
    fn test_detects_empty_arrow_function() {
        let test = TestProgram::for_rule(NoEmptyFunction);
        let result = test.lint_ast("test.ds", "const foo = () => {}");
        test.result(result).assert_lint("no-empty-function");
    }

    #[test]
    fn test_allows_function_with_body() {
        let test = TestProgram::for_rule(NoEmptyFunction);
        let result = test.lint_ast("test.ds", "function foo() { return 1; }");
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_allows_function_with_comment() {
        let test = TestProgram::for_rule(NoEmptyFunction);
        let result = test.lint_ast("test.ds", "function foo() { /* intentionally empty */ }");
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_allows_function_declaration_without_body() {
        let test = TestProgram::for_rule(NoEmptyFunction);
        let result = test.lint_ast("test.ts", "declare function foo(): void;");
        test.result(result).assert_no_lint("no-empty-function");
    }
}
