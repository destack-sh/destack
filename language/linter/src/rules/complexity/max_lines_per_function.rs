use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of lines per function.
    ///
    /// Long functions are harder to understand, test, and maintain.
    /// Consider breaking them into smaller, focused helper functions.
    #[lint(
        id = "max-lines-per-function",
        code = "LX007",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxLinesPerFunction,
    "Limit lines per function"
}

impl LintRule for MaxLinesPerFunction {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxLinesPerFunction::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let max_lines = ctx.options.max_lines_per_function;
        let file = ctx.program.files.get(ctx.module.file_id);

        for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Declaration(declaration_id) = ctx.tree.get(expression_id) else {
                continue;
            };

            let declaration = ctx.tree.get(*declaration_id);
            let ast::Declaration::Function { body, .. } = declaration else {
                continue;
            };

            // skip functions without bodies (declarations)
            let Some(body_id) = body else {
                continue;
            };

            // get span of the entire function declaration
            let span = ctx.tree.get_span(expression_id);

            // convert byte positions to line numbers
            let Some((start_line, _)) = file.get_position(span.start) else {
                continue;
            };
            let Some((end_line, _)) = file.get_position(span.end) else {
                continue;
            };

            // line count is inclusive (line 1 to line 3 = 3 lines)
            let line_count = (end_line - start_line + 1) as usize;
            if line_count > max_lines {
                let severity = ctx.get_effective_severity(meta, expression_id);
                if !severity.is_enabled() {
                    continue;
                }
                // get name from the function body's block span for more accurate location
                let body_span = ctx.tree.get_span(*body_id);
                ctx.report(
                    LintDiagnostic::new(
                        MAX_LINES_PER_FUNCTION.id,
                        MAX_LINES_PER_FUNCTION.code,
                        MAX_LINES_PER_FUNCTION.category,
                        severity,
                        format!("function has {line_count} lines (max {max_lines})"),
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
    fn test_detects_long_function() {
        let test = TestProgram::for_rule_without_builtins(MaxLinesPerFunction);
        // create a function with 51 lines (over default 50)
        let mut source = String::from("function foo() {\n");
        for i in 0..49 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("}\n");
        let result = test.lint_ast("test.ds", &source);
        test.result(result).assert_lint("max-lines-per-function");
    }

    #[test]
    fn test_allows_short_function() {
        let test = TestProgram::for_rule_without_builtins(MaxLinesPerFunction);
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
        test.result(result).assert_no_lint("max-lines-per-function");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_builtins(MaxLinesPerFunction);
        // create a function with exactly 50 lines
        let mut source = String::from("function foo() {\n");
        for i in 0..48 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("}\n");
        let result = test.lint_ast("test.ds", &source);
        test.result(result).assert_no_lint("max-lines-per-function");
    }

    #[test]
    fn test_checks_arrow_functions() {
        let test = TestProgram::for_rule_without_builtins(MaxLinesPerFunction);
        // create an arrow function with many lines
        let mut source = String::from("const foo = () => {\n");
        for i in 0..49 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("};\n");
        let result = test.lint_ast("test.ds", &source);
        test.result(result).assert_lint("max-lines-per-function");
    }

    #[test]
    fn test_ignores_function_declarations() {
        let test = TestProgram::for_rule_without_builtins(MaxLinesPerFunction);
        let result = test.lint_ast(
            "test.ds",
            r#"
declare function foo(): void;
"#,
        );
        test.result(result).assert_no_lint("max-lines-per-function");
    }
}
