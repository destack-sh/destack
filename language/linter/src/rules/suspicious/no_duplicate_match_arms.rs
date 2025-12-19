use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expressions_equal;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Warn on match arms with identical bodies.
    ///
    /// Having multiple match arms with the same body is often a sign of copy-paste
    /// errors or missed opportunities to combine patterns. Consider using `|` to
    /// combine patterns or extracting the common logic.
    #[lint(
        id = "no-duplicate-match-arms",
        code = "LU012",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoDuplicateMatchArms,
    "Warn on match arms with identical bodies"
}

impl LintRule for NoDuplicateMatchArms {
    fn meta(&self) -> &'static crate::LintMeta {
        NoDuplicateMatchArms::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Match { cases, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            // collect bodies and check for duplicates
            let mut seen_bodies: Vec<(
                ast::LocalNodeId<ast::MatchCase>,
                ast::LocalNodeId<ast::Expression>,
            )> = Vec::new();

            for case_id in cases {
                let case = ctx.tree.get(*case_id);

                // extract body expression from match case
                let body_id = match case {
                    ast::MatchCase::Expression { body, .. } => *body,
                    ast::MatchCase::Block { body, .. } => {
                        // for block bodies, check if it's a single expression block
                        let block = ctx.tree.get(*body);
                        if block.expressions.len() == 1 {
                            block.expressions[0]
                        } else {
                            // can't easily compare complex blocks
                            continue;
                        }
                    }
                };

                // check against previously seen bodies
                let is_duplicate = seen_bodies
                    .iter()
                    .any(|(_, prev_body)| expressions_equal(ctx, *prev_body, body_id));
                if is_duplicate {
                    let severity = ctx.get_effective_severity(meta, body_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    ctx.report(
                        LintDiagnostic::new(
                            NO_DUPLICATE_MATCH_ARMS.id,
                            NO_DUPLICATE_MATCH_ARMS.code,
                            NO_DUPLICATE_MATCH_ARMS.category,
                            severity,
                            "duplicate match arm body",
                            ctx.module.file_id,
                            ctx.tree.get_span(*case_id),
                        )
                        .with_label("this arm has the same body as a previous arm"),
                    );
                } else {
                    seen_bodies.push((*case_id, body_id));
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
    fn test_detects_duplicate_match_arms() {
        let test = TestProgram::for_rule(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 => foo()
    2 => foo()
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-match-arms");
    }

    #[test]
    fn test_allows_different_bodies() {
        let test = TestProgram::for_rule(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 => foo()
    2 => bar()
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-match-arms");
    }

    #[test]
    fn test_detects_duplicate_literals() {
        let test = TestProgram::for_rule(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 => 42
    2 => 42
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-match-arms");
    }
}
