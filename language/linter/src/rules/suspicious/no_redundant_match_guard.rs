use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::constant_to_bool;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow match guards that are always true or false.
    ///
    /// A match guard like `if true` or `if false` is redundant or makes the
    /// arm unreachable. If the guard is always true, remove it. If always
    /// false, the arm will never match and should be removed.
    #[lint(
        id = "no-redundant-match-guard",
        code = "LU016",
        category = Suspicious,
        level = Ast,
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoRedundantMatchGuard,
    "Disallow match guards that are always true or false"
}

impl LintRule for NoRedundantMatchGuard {
    fn meta(&self) -> &'static crate::LintMeta {
        NoRedundantMatchGuard::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::MatchCase>() {
            let match_case = ctx.tree.get(node_id);

            let guard_id = match match_case {
                ast::MatchCase::Expression { guard, .. } => guard,
                ast::MatchCase::Block { guard, .. } => guard,
            };

            let Some(guard_id) = guard_id else {
                continue;
            };

            let guard_expr = ctx.tree.get(*guard_id);
            let Some(is_truthy) = constant_to_bool(ctx, guard_expr) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let (message, label) = if is_truthy {
                ("match guard is always true", "this guard can be removed")
            } else {
                ("match guard is always false", "this arm will never match")
            };

            ctx.report(
                LintDiagnostic::new(
                    NO_REDUNDANT_MATCH_GUARD.id,
                    NO_REDUNDANT_MATCH_GUARD.code,
                    NO_REDUNDANT_MATCH_GUARD.category,
                    severity,
                    message,
                    ctx.module.file_id,
                    ctx.tree.get_span(*guard_id),
                )
                .with_label(label),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_guard_true() {
        let test = TestProgram::for_rule_without_builtins(NoRedundantMatchGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 if true => "one"
    _ => "other"
}
"#,
        );
        test.result(result).assert_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_detects_guard_false() {
        let test = TestProgram::for_rule_without_builtins(NoRedundantMatchGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 if false => "one"
    _ => "other"
}
"#,
        );
        test.result(result).assert_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_detects_guard_not_false() {
        let test = TestProgram::for_rule_without_builtins(NoRedundantMatchGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 if !false => "one"
    _ => "other"
}
"#,
        );
        test.result(result).assert_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_detects_guard_zero() {
        let test = TestProgram::for_rule_without_builtins(NoRedundantMatchGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 if 0 => "one"
    _ => "other"
}
"#,
        );
        test.result(result).assert_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_detects_guard_one() {
        let test = TestProgram::for_rule_without_builtins(NoRedundantMatchGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 if 1 => "one"
    _ => "other"
}
"#,
        );
        test.result(result).assert_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_allows_variable_guard() {
        let test = TestProgram::for_rule_without_builtins(NoRedundantMatchGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 if y > 0 => "one"
    _ => "other"
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_allows_no_guard() {
        let test = TestProgram::for_rule_without_builtins(NoRedundantMatchGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
match (x) {
    1 => "one"
    _ => "other"
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-redundant-match-guard");
    }
}
