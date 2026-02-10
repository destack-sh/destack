use destack_ast::{self as ast, Pattern};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce a specific name for caught errors.
    ///
    /// Consistent naming of caught errors improves code readability.
    /// Configure the expected name via `catch_error_name` option.
    #[lint(
        id = "catch-error-name",
        code = "LY001",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub CatchErrorName,
    "Enforce consistent catch error naming"
}

impl LintRule for CatchErrorName {
    fn meta(&self) -> &'static crate::LintMeta {
        CatchErrorName::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let expected_name = &ctx.options.catch_error_name;

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for try expressions with catch patterns
            let ast::Expression::Try {
                catch_pattern: Some(pattern_id),
                catch_expression,
                ..
            } = expr
            else {
                continue;
            };

            let pattern = ctx.tree.get(*pattern_id);

            // extract the binding name from the pattern
            let actual_name = match pattern {
                Pattern::Binding { name, .. } => Some(*name),
                _ => None,
            };

            if let Some(name_id) = actual_name {
                let actual = ctx.strings.get(name_id);
                if actual.as_ref() != expected_name {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let mut diagnostic = LintDiagnostic::new(
                        CATCH_ERROR_NAME.id,
                        CATCH_ERROR_NAME.code,
                        CATCH_ERROR_NAME.category,
                        severity,
                        format!(
                            "catch error should be named `{}`, not `{}`",
                            expected_name,
                            actual.as_ref()
                        ),
                        ctx.module.file_id,
                        ctx.tree.get_span(*pattern_id),
                    )
                    .with_label(format!("rename to `{expected_name}`"));

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) = catch_error_name_fix(
                            ctx,
                            *pattern_id,
                            name_id,
                            *catch_expression,
                            expected_name,
                        )
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// Build a safe fix by renaming the catch binding and aliasing the original name.
fn catch_error_name_fix(
    ctx: &LintModuleAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
    actual_name_id: destack_base::StringId,
    catch_expression: Option<ast::LocalNodeId<ast::Expression>>,
    expected_name: &str,
) -> Option<LintFix> {
    let pattern = ctx.tree.get(pattern_id);
    let Pattern::Binding {
        mutability,
        pattern: nested_pattern,
        ..
    } = pattern
    else {
        return None;
    };

    // keep simple binding patterns only
    if mutability.is_some() || nested_pattern.is_some() {
        return None;
    }

    // keep block catch expressions only
    let catch_expression_id = catch_expression?;
    let catch_expression = ctx.tree.get(catch_expression_id);
    if !matches!(catch_expression, ast::Expression::Block(_)) {
        return None;
    }

    let actual_name = ctx.strings.get(actual_name_id);
    if actual_name.as_ref() == expected_name {
        return None;
    }

    let block_span = ctx.tree.get_span(catch_expression_id);
    let block_text = ctx.get_span_text(block_span);
    let block_start_offset = block_text.find('{')? as u32;
    let insert_position = block_span.start + block_start_offset + 1;
    let alias_statement = format!("\n    let {} = {expected_name};", actual_name.as_ref());

    let pattern_span = ctx.tree.get_span(pattern_id);
    let edits = ctx
        .edit_builder()
        .replace(pattern_span, expected_name)
        .insert(insert_position, alias_statement)
        .into_edits();

    Some(LintFix::safe("Rename catch binding and preserve old name alias").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_wrong_error_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_ast(
            "catch_error_name/test_detects_wrong_error_name.ds",
            r#"
try {
    doSomething()
} catch (e) {
    console.log(e)
}
"#,
        );
        test.result(result)
            .assert_lint("catch-error-name")
            .assert_has_fix("catch-error-name");
    }

    #[test]
    fn test_allows_correct_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_ast(
            "catch_error_name/test_allows_correct_name.ds",
            r#"
try {
    doSomething()
} catch (error) {
    console.log(error)
}
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    #[test]
    fn test_allows_try_without_catch() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_ast(
            "catch_error_name/test_allows_try_without_catch.ds",
            r#"
try {
    doSomething()
} finally {
    cleanup()
}
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    #[test]
    fn test_detects_err_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_ast(
            "catch_error_name/test_detects_err_name.ds",
            r#"
try {
    fetch()
} catch (err) {
    console.error(err)
}
"#,
        );
        test.result(result).assert_lint("catch-error-name");
    }

    #[test]
    fn test_fix_renames_catch_binding_and_aliases_old_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_ast(
            "catch_error_name/test_fix_renames_catch_binding_and_aliases_old_name.ds",
            r#"
try {
    run()
} catch (err) {
    console.log(err)
}
"#,
        );
        test.result(result)
            .assert_lint("catch-error-name")
            .assert_safe_fixed(
                r#"
try {
    run()
} catch (error) {
    let err = error;
    console.log(err)
}
"#,
            );
    }

    #[test]
    fn test_no_fix_for_non_binding_catch_pattern() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_ast(
            "catch_error_name/test_no_fix_for_non_binding_catch_pattern.ds",
            r#"
try {
    run()
} catch ({ reason }) {
    console.log(reason)
}
"#,
        );
        test.result(result)
            .assert_no_lint("catch-error-name")
            .assert_has_no_fix("catch-error-name");
    }
}
