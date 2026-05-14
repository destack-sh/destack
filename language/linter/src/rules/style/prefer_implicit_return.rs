use crate::LintMeta;
use destack_dir::{self as dir, Block, Declaration, Expression, FunctionForm};
use destack_workspace::LintSeverity;

use crate::rules::common::source_text_contains_comment_token;
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer implicit return for simple arrow functions.
    ///
    /// Use `() => x` instead of `() => { return x }` for concise arrow functions.
    #[lint(
        id = "prefer-implicit-return",
        code = "LY041",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferImplicitReturn,
    "Prefer implicit return for arrow functions"
}

impl LintRule for PreferImplicitReturn {
    fn meta(&self) -> &'static LintMeta {
        PreferImplicitReturn::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            let decl = ctx.dir.get(node_id);

            // look for arrow functions
            let Declaration::Function(declaration) = decl else {
                continue;
            };
            let Some(body_id) = declaration.body else {
                continue;
            };

            // only check lambda functions
            if declaration.signature.form != FunctionForm::Lambda {
                continue;
            }

            let body_expr = ctx.dir.get(body_id);

            // check if body is a block with single return statement
            let Expression::Block(block_id) = body_expr else {
                continue;
            };

            let block = ctx.dir.get(*block_id);

            // extract the returned expression for implicit body replacement
            let Some(return_value_id) = single_return_block_value(ctx.dir.tree(), block) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build concise expression body replacement
            let body_span = ctx.dir.get_span(body_id);
            let body_text = ctx.get_span_text(body_span);
            let return_value_span = ctx.dir.get_span(return_value_id);
            let return_value = ctx.dir.get(return_value_id);
            let mut replacement = ctx.get_span_text(return_value_span).to_string();
            if matches!(return_value, Expression::ObjectExpression { .. }) {
                replacement = format!("({replacement})");
            }
            let maybe_fix = if source_text_contains_comment_token(body_text) {
                None
            } else {
                let edits = ctx
                    .edit_builder()
                    .replace(body_span, replacement)
                    .into_edits();
                Some(LintFix::safe("Use concise implicit return").with_edits(edits))
            };

            let diagnostic = LintReport::new(
                PREFER_IMPLICIT_RETURN.id,
                PREFER_IMPLICIT_RETURN.code,
                PREFER_IMPLICIT_RETURN.category,
                severity,
                "use implicit return instead of block with return",
                ctx.dir.get_span(body_id),
            )
            .label("use `() => x` instead of `() => { return x }`");
            let diagnostic = if let Some(fix) = maybe_fix {
                diagnostic.fix(fix)
            } else {
                diagnostic
            };

            ctx.report(diagnostic);
        }
    }
}

/// Return the value expression id when a block has one `return value` statement.
fn single_return_block_value(
    tree: &dir::Tree,
    block: &Block,
) -> Option<dir::LocalNodeId<Expression>> {
    if block.len() != 1 {
        return None;
    }

    let expr_id = block.first_expression()?;
    let expr = tree.get(expr_id);

    // check if it's a return with a value
    let Expression::Return {
        value: Some(value_id),
        ..
    } = expr
    else {
        return None;
    };

    Some(*value_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_block_with_return() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        let result = test.lint(
            "prefer_implicit_return/test_detects_block_with_return.ds",
            r#"
const double = (x) => { return x * 2 }
"#,
        );
        test.result(result).assert_lint("prefer-implicit-return");
    }

    #[test]
    fn test_detected_block_with_return_has_fix() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        let result = test.lint(
            "prefer_implicit_return/test_detected_block_with_return_has_fix.ds",
            r#"
const double = (x) => { return x * 2 }
"#,
        );
        test.result(result)
            .assert_lint("prefer-implicit-return")
            .assert_has_fix("prefer-implicit-return");
    }

    #[test]
    fn test_fix_block_with_return() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        let result = test.lint(
            "prefer_implicit_return/test_fix_block_with_return.ds",
            r#"
const double = (x) => { return x * 2 }
"#,
        );
        test.result(result)
            .assert_lint("prefer-implicit-return")
            .assert_safe_fixed(
                r#"
const double = (x) => x * 2;
"#,
            );
    }

    #[test]
    fn test_fix_block_with_object_return() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        let result = test.lint(
            "prefer_implicit_return/test_fix_block_with_object_return.ds",
            r#"
const value = () => { return { ok: true } }
"#,
        );
        test.result(result)
            .assert_lint("prefer-implicit-return")
            .assert_safe_fixed(
                r#"
const value = () => ({ ok: true });
"#,
            );
    }

    #[test]
    fn test_fix_block_with_await_return() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        let result = test.lint(
            "prefer_implicit_return/test_fix_block_with_await_return.ds",
            r#"
const load = async () => { return await fetchValue() }
"#,
        );
        test.result(result)
            .assert_lint("prefer-implicit-return")
            .assert_has_fix("prefer-implicit-return")
            .assert_safe_fixed(
                r#"
const load = async () => await fetchValue();
"#,
            );
    }

    #[test]
    fn test_allows_implicit_return() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        let result = test.lint(
            "prefer_implicit_return/test_allows_implicit_return.ds",
            r#"
const double = (x) => x * 2
"#,
        );
        test.result(result).assert_no_lint("prefer-implicit-return");
    }

    #[test]
    fn test_allows_multi_statement_block() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        // multi-statement blocks can't use implicit return
        let result = test.lint(
            "prefer_implicit_return/test_allows_multi_statement_block.ds",
            r#"
const double = (x) => {
    const y = x * 2
    return y
}
"#,
        );
        test.result(result).assert_no_lint("prefer-implicit-return");
    }

    #[test]
    fn test_allows_function_declaration() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        // traditional functions always need blocks
        let result = test.lint(
            "prefer_implicit_return/test_allows_function_declaration.ds",
            r#"
function double(x) {
    return x * 2
}
"#,
        );
        test.result(result).assert_no_lint("prefer-implicit-return");
    }

    #[test]
    fn test_allows_void_return() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        // void returns can't be implicit
        let result = test.lint(
            "prefer_implicit_return/test_allows_void_return.ds",
            r#"
const log = (x) => { return }
"#,
        );
        test.result(result).assert_no_lint("prefer-implicit-return");
    }

    #[test]
    fn test_no_fix_when_return_block_contains_comment() {
        let test = TestProgram::for_rule_without_prelude(PreferImplicitReturn);
        let result = test.lint(
            "prefer_implicit_return/test_no_fix_when_return_block_contains_comment.ds",
            r#"
const value = () => {
    // keep this explanation
    return 1
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-implicit-return")
            .assert_has_no_fix("prefer-implicit-return");
    }
}
