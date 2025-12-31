use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of lines per file.
    ///
    /// Files with many lines are harder to navigate and maintain.
    /// Consider splitting large files into smaller, focused modules.
    /// Default maximum is 500 lines.
    #[lint(
        id = "max-lines",
        code = "LX006",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxLines,
    "Limit lines per file"
}

impl LintRule for MaxLines {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxLines::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let max_lines = ctx.options.max_lines;
        let file = ctx.program.files.get(ctx.module.file_id);
        let line_count = file.line_count() as usize;
        if line_count > max_lines {
            // file-level lint: use first root expression for severity check, or base severity
            let severity = ctx
                .roots
                .first()
                .map(|root| ctx.get_effective_severity(meta, *root))
                .unwrap_or_else(|| ctx.get_severity(meta));
            if !severity.is_enabled() {
                return;
            }
            // report at the first line of the file
            let span = destack_source::Span::new(ctx.module.file_id, 0, 0);
            ctx.report(
                LintDiagnostic::new(
                    MAX_LINES.id,
                    MAX_LINES.code,
                    MAX_LINES.category,
                    severity,
                    format!("file has {line_count} lines (max {max_lines})"),
                    ctx.module.file_id,
                    span,
                )
                .with_label("consider splitting into smaller modules"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_lines() {
        let test = TestProgram::for_rule_without_builtins(MaxLines);
        // create a file with 501 lines (over default 500)
        let mut source = String::new();
        for i in 0..501 {
            source.push_str(&format!("let x{i} = {i};\n"));
        }
        let result = test.lint_ast("test.ds", &source);
        test.result(result).assert_lint("max-lines");
    }

    #[test]
    fn test_allows_small_file() {
        let test = TestProgram::for_rule_without_builtins(MaxLines);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1;
let y = 2;
let z = 3;
"#,
        );
        test.result(result).assert_no_lint("max-lines");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_builtins(MaxLines);
        // create a file with exactly 500 lines (499 with newlines + 1 trailing)
        let mut source = String::new();
        for i in 0..499 {
            source.push_str(&format!("let x{i} = {i};\n"));
        }
        // last line without trailing newline to get exactly 500 lines
        source.push_str("let x499 = 499;");
        let result = test.lint_ast("test.ds", &source);
        test.result(result).assert_no_lint("max-lines");
    }
}
