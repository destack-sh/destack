use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow specified warning terms in comments.
    ///
    /// Warning comments like TODO, FIXME, and HACK indicate incomplete work.
    /// Resolve these before committing or track them in an issue tracker.
    #[lint(
        id = "no-warning-comments",
        code = "LR019",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoWarningComments,
    "Disallow warning comments"
}

impl LintRule for NoWarningComments {
    fn meta(&self) -> &'static crate::LintMeta {
        NoWarningComments::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let warning_terms = &ctx.options.warning_comment_terms;

        // iterate over all annotations (comments)
        for node_id in ctx.tree.iter_nodes::<ast::Annotation>() {
            let annotation = ctx.tree.get(node_id);
            let ast::Annotation::Comment { node, .. } = annotation else {
                continue;
            };

            // get the actual Comment node and its string content
            let comment = ctx.tree.get(*node);
            let comment_text = ctx.strings.get(comment.string);
            let comment_upper = comment_text.to_uppercase();

            // check for warning terms in the comment
            for term in warning_terms {
                if comment_upper.contains(&term.to_uppercase()) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        break;
                    }
                    ctx.report(
                        LintDiagnostic::new(
                            NO_WARNING_COMMENTS.id,
                            NO_WARNING_COMMENTS.code,
                            NO_WARNING_COMMENTS.category,
                            severity,
                            format!("warning comment contains `{term}`"),
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("resolve before committing"),
                    );
                    break; // only report once per comment
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_todo_comment() {
        let test = TestProgram::for_rule(NoWarningComments);
        let result = test.lint_ast("test.ts", "// TODO: fix this");
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_detects_fixme_comment() {
        let test = TestProgram::for_rule(NoWarningComments);
        let result = test.lint_ast("test.ts", "// FIXME: broken");
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_detects_hack_comment() {
        let test = TestProgram::for_rule(NoWarningComments);
        let result = test.lint_ast("test.ts", "/* HACK: temporary workaround */");
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_case_insensitive() {
        let test = TestProgram::for_rule(NoWarningComments);
        let result = test.lint_ast("test.ts", "// todo: lowercase");
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_allows_normal_comment() {
        let test = TestProgram::for_rule(NoWarningComments);
        let result = test.lint_ast("test.ts", "// this is a regular comment");
        test.result(result).assert_no_lint("no-warning-comments");
    }
}
