use crate::LintMeta;
use destack_ast as ast;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, callable_owner_span, for_each_callable_signature, span_has_comment,
};
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow redundant return statements at the end of functions.
    ///
    /// A `return` statement with no value at the end of a function is redundant
    /// since the function would return anyway.
    #[lint(
        id = "no-useless-return",
        code = "LU041",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessReturn,
    "Disallow useless return statements"
}

impl LintRule for NoUselessReturn {
    fn meta(&self) -> &'static LintMeta {
        NoUselessReturn::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect all callable bodies consistently
        for_each_callable_signature(ctx.tree, |owner_id, _signature, body_expression_id| {
            let Some(body_id) = body_expression_id else {
                return;
            };

            report_trailing_bare_return(ctx, meta, owner_id, body_id);
        });
    }
}

/// Report one redundant trailing bare return inside a callable body.
fn report_trailing_bare_return(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
    owner_id: CallableOwnerId,
    body_expression_id: ast::LocalNodeId<ast::Expression>,
) {
    // keep only bare returns at the end of the callable body
    let Some(return_id) = get_trailing_bare_return(ctx, body_expression_id) else {
        return;
    };

    let severity = match owner_id {
        CallableOwnerId::Declaration(declaration_id) => {
            ctx.get_effective_severity(meta, declaration_id)
        }
        CallableOwnerId::Member(member_id) => ctx.get_effective_severity(meta, member_id),
        CallableOwnerId::Property(property_id) => ctx.get_effective_severity(meta, property_id),
    };
    if !severity.is_enabled() {
        return;
    }

    let return_span = ctx.tree.get_span(return_id);
    let mut diagnostic = LintReport::new(
        NO_USELESS_RETURN.id,
        NO_USELESS_RETURN.code,
        NO_USELESS_RETURN.category,
        severity,
        "useless return statement",
        callable_owner_span(ctx.tree, owner_id),
    )
    .label("this return is unnecessary");

    // avoid deleting commented returns
    if ctx.compute_fixes
        && !span_has_comment(ctx.tree, return_span)
        && !return_has_trailing_comment(ctx, return_span)
    {
        let edits = ctx.edit_builder().delete(return_span).into_edits();
        let fix = LintFix::safe("Remove useless return").with_edits(edits);
        diagnostic = diagnostic.fix(fix);
    }

    ctx.report(diagnostic);
}

/// Return true when source text has a trailing comment after one return span.
fn return_has_trailing_comment(ctx: &LintAstContext<'_>, return_span: Span) -> bool {
    let source = ctx.source_text().as_bytes();
    let mut cursor = return_span.end as usize;
    while cursor < source.len() && (source[cursor] == b' ' || source[cursor] == b'\t') {
        cursor += 1;
    }

    cursor + 1 < source.len()
        && source[cursor] == b'/'
        && (source[cursor + 1] == b'/' || source[cursor + 1] == b'*')
}

/// Get a trailing bare return statement from a function body.
fn get_trailing_bare_return(
    ctx: &LintAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            if let Some(last_id) = block.last_expression() {
                return get_trailing_bare_return(ctx, last_id);
            }
            None
        }
        ast::Expression::Return { value } => {
            if value.is_none() {
                Some(expr_id)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_bare_return_at_end() {
        let test = TestProgram::for_rule_without_prelude(NoUselessReturn);
        let result = test.lint_ast(
            "no_useless_return/test_detects_bare_return_at_end.ds",
            r#"
function foo() {
    bar()
    return
}
"#,
        );
        test.result(result).assert_lint("no-useless-return");
    }

    #[test]
    fn test_allows_return_with_value() {
        let test = TestProgram::for_rule_without_prelude(NoUselessReturn);
        let result = test.lint_ast(
            "no_useless_return/test_allows_return_with_value.ds",
            r#"
function foo() {
    return 42;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-return");
    }

    #[test]
    fn test_allows_no_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessReturn);
        let result = test.lint_ast(
            "no_useless_return/test_allows_no_return.ds",
            r#"
function foo() {
    console.log("hello");
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-return");
    }

    #[test]
    fn test_allows_early_return() {
        // early return is not useless
        let test = TestProgram::for_rule_without_prelude(NoUselessReturn);
        let result = test.lint_ast(
            "no_useless_return/test_allows_early_return.ds",
            r#"
function foo(x: boolean) {
    if (x) {
        return;
    }
    console.log("continuing");
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-return");
    }

    #[test]
    fn test_fix_removes_useless_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessReturn);
        let result = test.lint_ast(
            "no_useless_return/test_fix_removes_useless_return.ds",
            r#"
function foo() {
    bar()
    return
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-return")
            .assert_safe_fixed(
                r#"
function foo() {
    bar()
}
"#,
            );
    }

    #[test]
    fn test_no_fix_when_return_has_comment_trivia() {
        let test = TestProgram::for_rule_without_prelude(NoUselessReturn);
        let result = test.lint_ast(
            "no_useless_return/test_no_fix_when_return_has_comment_trivia.ds",
            r#"
function foo() {
    bar()
    return // keep intent
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-return")
            .assert_has_no_fix("no-useless-return");
    }

    #[test]
    fn test_detects_bare_return_at_end_of_method() {
        let test = TestProgram::for_rule_without_prelude(NoUselessReturn);
        let result = test.lint_ast(
            "no_useless_return/test_detects_bare_return_at_end_of_method.ds",
            r#"
class Foo {
    run() {
        bar()
        return
    }
}
"#,
        );
        test.result(result).assert_lint("no-useless-return");
    }

    #[test]
    fn test_detects_bare_return_at_end_of_object_method() {
        let test = TestProgram::for_rule_without_prelude(NoUselessReturn);
        let result = test.lint_ast(
            "no_useless_return/test_detects_bare_return_at_end_of_object_method.ds",
            r#"
const service = {
    run() {
        bar()
        return
    }
}
"#,
        );
        test.result(result).assert_lint("no-useless-return");
    }
}
