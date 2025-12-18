use destack_ast::{self as ast, Expression, MatchCase, Pattern, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest using if/else instead of match on booleans.
    ///
    /// Matching on boolean values with `true` and `false` arms is more
    /// idiomatically expressed as an if/else statement.
    ///
    /// ```
    /// // bad
    /// match condition {
    ///     true => doX()
    ///     false => doY()
    /// }
    ///
    /// // good
    /// if condition {
    ///     doX()
    /// } else {
    ///     doY()
    /// }
    /// ```
    #[lint(
        id = "prefer-if-else-over-match-bool",
        code = "LY054",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferIfElseOverMatchBool,
    "Prefer if/else over match on boolean"
}

impl LintRule for PreferIfElseOverMatchBool {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferIfElseOverMatchBool::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let ast::Expression::Match { cases, .. } = expression else {
                continue;
            };

            // look for exactly 2 cases
            if cases.len() != 2 {
                continue;
            }

            // check if we have true/false or false/true patterns
            let first_pattern = get_case_pattern(ctx, cases[0]);
            let second_pattern = get_case_pattern(ctx, cases[1]);

            let is_bool_match = match (first_pattern, second_pattern) {
                (Some(first_bool), Some(second_bool)) => first_bool != second_bool,
                _ => false,
            };

            if !is_bool_match {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    PREFER_IF_ELSE_OVER_MATCH_BOOL.id,
                    PREFER_IF_ELSE_OVER_MATCH_BOOL.code,
                    PREFER_IF_ELSE_OVER_MATCH_BOOL.category,
                    severity,
                    "use `if/else` instead of `match` on boolean",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("replace with if/else"),
            );
        }
    }
}

/// Get the boolean literal value from a match case pattern, if it is one.
fn get_case_pattern(
    ctx: &LintModuleAstContext<'_>,
    case_id: ast::LocalNodeId<MatchCase>,
) -> Option<bool> {
    let case = ctx.tree.get(case_id);

    let pattern_id = match case {
        MatchCase::Block { pattern, .. } | MatchCase::Expression { pattern, .. } => *pattern,
    };

    let pattern = ctx.tree.get(pattern_id);

    // check if it's an expression pattern with a boolean literal
    let Pattern::Expression { value } = pattern else {
        return None;
    };

    let expression = ctx.tree.get(*value);

    match expression {
        Expression::ScalarLiteral(ScalarLiteral::Boolean(b)) => Some(*b),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_match_bool_detected() {
        let test = TestProgram::for_rule(PreferIfElseOverMatchBool);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: bool) {
    match condition {
        true => doX()
        false => doY()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_bool_false_first_detected() {
        let test = TestProgram::for_rule(PreferIfElseOverMatchBool);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: bool) {
    match condition {
        false => doY()
        true => doX()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_if_else_allowed() {
        let test = TestProgram::for_rule(PreferIfElseOverMatchBool);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: bool) {
    if condition {
        doX()
    } else {
        doY()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_non_bool_allowed() {
        let test = TestProgram::for_rule(PreferIfElseOverMatchBool);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    match x {
        1 => doX()
        _ => doY()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_enum_allowed() {
        let test = TestProgram::for_rule(PreferIfElseOverMatchBool);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32?) {
    match x {
        Some(n) => doX(n)
        None => doY()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_bool_with_wildcard_allowed() {
        let test = TestProgram::for_rule(PreferIfElseOverMatchBool);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: bool) {
    match condition {
        true => doX()
        _ => doY()
    }
}
"#,
        );
        // using wildcard instead of explicit false is fine
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_more_than_two_arms_allowed() {
        let test = TestProgram::for_rule(PreferIfElseOverMatchBool);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    match x {
        1 => doX()
        2 => doY()
        3 => doZ()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_bool_same_value_allowed() {
        let test = TestProgram::for_rule(PreferIfElseOverMatchBool);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: bool) {
    match condition {
        true => doX()
        true => doY()
    }
}
"#,
        );
        // both arms matching true doesn't make sense as if/else
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }
}
