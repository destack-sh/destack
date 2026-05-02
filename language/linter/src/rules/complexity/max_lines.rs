use crate::LintMeta;
use destack_workspace::LintSeverity;

use crate::rules::common::count_file_lines;
use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of lines per file.
    ///
    /// Files with many lines are harder to navigate and maintain.
    /// Consider splitting large files into smaller, focused modules.
    /// Default maximum is 500 lines.
    #[lint(
        id = "max-lines",
        code = "LX005",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxLines,
    "Limit lines per file"
}

impl LintRule for MaxLines {
    fn meta(&self) -> &'static LintMeta {
        MaxLines::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        // resolve lint metadata, threshold, and active options
        let meta = self.meta();
        let max_lines = ctx.options.complexity.max_lines;
        let skip_comments = ctx.options.complexity.max_lines_skip_comments;
        let skip_blank_lines = ctx.options.complexity.max_lines_skip_blank_lines;

        // count file lines with option aware filtering
        let file = ctx.file.as_ref();
        let line_count = count_file_lines(file, skip_comments, skip_blank_lines);
        if line_count <= max_lines {
            return;
        }

        // resolve file level severity from first root or base setting
        let severity = ctx
            .roots
            .first()
            .map(|root| ctx.get_effective_severity(meta, *root))
            .unwrap_or_else(|| ctx.get_severity(meta));
        if !severity.is_enabled() {
            return;
        }

        // report at file start for one file level diagnostic
        let span = destack_source::Span::new(ctx.module.file_id, 0, 0);
        ctx.report(
            LintReport::new(
                MAX_LINES.id,
                MAX_LINES.code,
                MAX_LINES.category,
                severity,
                format!("file has {line_count} lines (max {max_lines})"),
                span,
            )
            .label("consider splitting into smaller modules"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_lines() {
        let test = TestProgram::for_rule_without_prelude(MaxLines);

        // create a file with 501 lines, over default 500
        let mut source = String::new();
        for i in 0..501 {
            source.push_str(&format!("let x{i} = {i};\n"));
        }

        // verify lint report for oversized file
        let result = test.lint_ast("max_lines/test_detects_too_many_lines.ds", &source);
        test.result(result).assert_lint("max-lines");
    }

    #[test]
    fn test_allows_small_file() {
        let test = TestProgram::for_rule_without_prelude(MaxLines);

        // keep line count under default threshold
        let result = test.lint_ast(
            "max_lines/test_allows_small_file.ds",
            r#"
let x = 1;
let y = 2;
let z = 3;
"#,
        );

        // verify no lint for small file
        test.result(result).assert_no_lint("max-lines");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_prelude(MaxLines);

        // create a file with exactly 500 lines
        let mut source = String::new();
        for i in 0..499 {
            source.push_str(&format!("let x{i} = {i};\n"));
        }
        source.push_str("let x499 = 499;");

        // verify no lint at exact threshold
        let result = test.lint_ast("max_lines/test_allows_exactly_at_limit.ds", &source);
        test.result(result).assert_no_lint("max-lines");
    }

    #[test]
    fn test_skips_blank_lines_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(MaxLines).with_options(|options| {
            options.complexity.max_lines = 2;
            options.complexity.max_lines_skip_blank_lines = true;
        });

        // ignore blank lines while counting
        let result = test.lint_ast(
            "max_lines/test_skips_blank_lines_when_enabled.ds",
            r#"
let first = 1;

let second = 2;

"#,
        );

        // verify blank-line filtering prevents false positives
        test.result(result).assert_no_lint("max-lines");
    }

    #[test]
    fn test_skips_comment_lines_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(MaxLines).with_options(|options| {
            options.complexity.max_lines = 2;
            options.complexity.max_lines_skip_comments = true;
        });

        // ignore full-line comments while counting
        let result = test.lint_ast(
            "max_lines/test_skips_comment_lines_when_enabled.ds",
            r#"
// comment one
/*
comment two
*/
let first = 1;
let second = 2;
"#,
        );

        // verify comment-line filtering prevents false positives
        test.result(result).assert_no_lint("max-lines");
    }

    #[test]
    fn test_counts_code_with_trailing_comment_when_skipping_comments() {
        let test = TestProgram::for_rule_without_prelude(MaxLines).with_options(|options| {
            options.complexity.max_lines = 1;
            options.complexity.max_lines_skip_comments = true;
        });

        // still count lines that contain code before trailing comments
        let result = test.lint_ast(
            "max_lines/test_counts_code_with_trailing_comment_when_skipping_comments.ds",
            r#"
let first = 1; // trailing comment
let second = 2;
"#,
        );

        // verify code lines are not dropped by comment filtering
        test.result(result).assert_lint("max-lines");
    }
}
