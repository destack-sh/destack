use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of statements per function.
    ///
    /// Functions with many statements are harder to understand and maintain.
    /// Consider extracting logic into helper functions.
    #[lint(
        id = "max-statements",
        code = "LX006",
        category = Complexity,
        level = Ast
    )]
    pub MaxStatements,
    "Limit statements per function"
}

impl LintRule for MaxStatements {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxStatements::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let max_statements = ctx.options.max_statements;

        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            let ast::Declaration::Function { body, .. } = declaration else {
                continue;
            };

            let Some(body_id) = body else {
                continue;
            };

            // get the function body and count statements
            let body_expression = ctx.tree.get(*body_id);
            let statement_count = match body_expression {
                ast::Expression::Block(block_id) => {
                    let block = ctx.tree.get(*block_id);
                    block.expressions.len()
                }
                // single expression body counts as 1
                _ => 1,
            };

            if statement_count > max_statements {
                let body_span = ctx.tree.get_span(*body_id);
                ctx.report(
                    LintDiagnostic::new(
                        MAX_STATEMENTS.id,
                        MAX_STATEMENTS.code,
                        MAX_STATEMENTS.category,
                        severity,
                        format!("function has {statement_count} statements (max {max_statements})"),
                        ctx.module.file_id,
                        body_span,
                    )
                    .with_label("consider breaking into smaller functions"),
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
    fn test_detects_too_many_statements() {
        let test = TestProgram::for_rule(MaxStatements);
        // create a function with 51 statements (over default 50)
        let mut source = String::from("function foo() {\n");
        for i in 0..51 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("}\n");
        let result = test.lint_ast("test.ds", &source);
        test.result(result).assert_lint("max-statements");
    }

    #[test]
    fn test_allows_few_statements() {
        let test = TestProgram::for_rule(MaxStatements);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    let x = 1;
    let y = 2;
    return x + y;
}
"#,
        );
        test.result(result).assert_no_lint("max-statements");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule(MaxStatements);
        // create a function with exactly 50 statements
        let mut source = String::from("function foo() {\n");
        for i in 0..50 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("}\n");
        let result = test.lint_ast("test.ds", &source);
        test.result(result).assert_no_lint("max-statements");
    }
}
