use destack_source::Span;
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
    fn meta(&self) -> &'static crate::LintMeta {
        MaxLines::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // resolve lint metadata, threshold, and active options
        let meta = self.meta();
        let max_lines = ctx.options.max_lines;
        let skip_comments = ctx.options.max_lines_skip_comments;
        let skip_blank_lines = ctx.options.max_lines_skip_blank_lines;

        // count file lines with option aware filtering
        let file = ctx.program.files.get(ctx.module.file_id);
        let line_count = count_file_lines(&file, skip_comments, skip_blank_lines);
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
        let span = Span::new(ctx.module.file_id, 0, 0);
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

/// Count file lines with optional comment and blank line skipping.
fn count_file_lines(
    file: &destack_source::File,
    skip_comments: bool,
    skip_blank_lines: bool,
) -> usize {
    // initialize line counters and comment state
    let total_lines = file.line_count() as usize;
    let mut counted_lines = 0usize;
    let mut in_block_comment = false;

    // scan each source line with trailing empty line normalization
    for line_index in 0..total_lines {
        let Some(line_text) = file.get_line_str(line_index as u32) else {
            continue;
        };

        // ignore one synthetic trailing empty line for newline terminated files
        if line_index + 1 == total_lines && line_text.is_empty() {
            continue;
        }

        // skip full-line comments when configured
        if skip_comments && !line_has_code_outside_comments(line_text, &mut in_block_comment) {
            continue;
        }

        // skip blank lines when configured
        if skip_blank_lines && line_text.trim().is_empty() {
            continue;
        }

        counted_lines += 1;
    }

    counted_lines
}

/// Return true when one line contains non-comment code.
fn line_has_code_outside_comments(line_text: &str, in_block_comment: &mut bool) -> bool {
    // track whether this line contains any code token
    let mut has_code = false;
    let bytes = line_text.as_bytes();
    let mut index = 0usize;

    // scan one line while tracking block comment state
    while index < bytes.len() {
        // consume an active block comment until close token or line end
        if *in_block_comment {
            let Some(close_offset) = line_text[index..].find("*/") else {
                return has_code;
            };

            index += close_offset + 2;
            *in_block_comment = false;
            continue;
        }

        // skip leading and interstitial whitespace
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        // stop at one line comment marker
        if line_text[index..].starts_with("//") {
            break;
        }

        // start block comment mode when block opener is found
        if line_text[index..].starts_with("/*") {
            *in_block_comment = true;
            index += 2;
            continue;
        }

        // mark code and continue scanning to track trailing block comments
        has_code = true;
        index += 1;
    }

    has_code
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
            options.max_lines = 2;
            options.max_lines_skip_blank_lines = true;
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
            options.max_lines = 2;
            options.max_lines_skip_comments = true;
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
            options.max_lines = 1;
            options.max_lines_skip_comments = true;
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
