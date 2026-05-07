use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty destructuring patterns.
    ///
    /// Empty destructuring patterns like `const {} = obj` or `const [] = arr`
    /// don't bind any values and are likely mistakes.
    #[lint(
        id = "no-empty-pattern",
        code = "LU014",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoEmptyPattern,
    "Disallow empty destructuring patterns"
}

impl LintRule for NoEmptyPattern {
    fn meta(&self) -> &'static LintMeta {
        NoEmptyPattern::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Pattern>() {
            let pattern = ctx.tree.get(node_id);

            let is_empty = match pattern {
                ast::Pattern::Object { fields } => fields.is_empty(),
                ast::Pattern::Sequence { fields } => fields.is_empty(),
                ast::Pattern::Tuple { fields } => fields.is_empty(),
                _ => false,
            };
            if !is_empty {
                continue;
            }

            // allow parameter object patterns when configured
            if matches!(pattern, ast::Pattern::Object { .. })
                && ctx
                    .options
                    .correctness
                    .no_empty_pattern_allow_object_patterns_as_parameters
                && object_pattern_is_parameter_position(ctx, node_id)
            {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let kind = match pattern {
                ast::Pattern::Object { .. } => "object",
                ast::Pattern::Sequence { .. } => "sequence",
                ast::Pattern::Tuple { .. } => "tuple",
                _ => unreachable!(),
            };

            ctx.report({
                let mut diagnostic = LintReport::new(
                    NO_EMPTY_PATTERN.id,
                    NO_EMPTY_PATTERN.code,
                    NO_EMPTY_PATTERN.category,
                    severity,
                    format!("empty {kind} destructuring pattern"),
                    ctx.tree.get_span(node_id),
                )
                .label("this pattern doesn't bind any values");
                if ctx.compute_fixes
                    && let Some(fix) = no_empty_pattern_fix(ctx, node_id)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                diagnostic
            });
        }
    }
}

/// Return true when an object pattern is used as a callable parameter.
fn object_pattern_is_parameter_position(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> bool {
    let Some(parent_id) = ctx.parents.get(pattern_id) else {
        return false;
    };

    matches!(ctx.tree.get_node_type(parent_id), ast::NodeType::Parameter)
}

/// Build a safe replacement for empty declarator patterns.
fn no_empty_pattern_fix(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<LintFix> {
    // keep declaration patterns only
    let mut declarator_for_pattern = None;
    for declarator_id in ctx.tree.iter_nodes::<ast::Declarator>() {
        let declarator = ctx.tree.get(declarator_id);
        if declarator.pattern == pattern_id {
            declarator_for_pattern = Some(declarator_id);
            break;
        }
    }
    let declarator_id = declarator_for_pattern?;
    let declarator = ctx.tree.get(declarator_id);

    // keep declarators with initializer so rewrite preserves behavior
    declarator.value?;

    // replace the empty pattern with `_`
    let pattern_span = ctx.tree.get_span(pattern_id);
    let edits = ctx.edit_builder().replace(pattern_span, "_").into_edits();
    Some(LintFix::safe("Replace empty pattern with wildcard binding").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_object_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyPattern);
        let result = test.lint_ast(
            "no_empty_pattern/test_detects_empty_object_pattern.ds",
            r#"
const {} = obj
"#,
        );
        test.result(result)
            .assert_lint("no-empty-pattern")
            .assert_safe_fixed(
                r#"
const _ = obj;
"#,
            );
    }

    #[test]
    fn test_detects_empty_array_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyPattern);
        let result = test.lint_ast(
            "no_empty_pattern/test_detects_empty_array_pattern.ds",
            r#"
const [] = arr
"#,
        );
        test.result(result)
            .assert_lint("no-empty-pattern")
            .assert_safe_fixed(
                r#"
const _ = arr;
"#,
            );
    }

    #[test]
    fn test_detects_empty_tuple_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyPattern);
        let result = test.lint_ast(
            "no_empty_pattern/test_detects_empty_tuple_pattern.ds",
            r#"
const () = tuple
"#,
        );
        test.result(result)
            .assert_lint("no-empty-pattern")
            .assert_safe_fixed(
                r#"
const _ = tuple;
"#,
            );
    }

    #[test]
    fn test_detects_empty_pattern_in_function_param() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyPattern);
        let result = test.lint_ast(
            "no_empty_pattern/test_detects_empty_pattern_in_function_param.ds",
            r#"
function foo({}) {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-pattern")
            .assert_has_no_fix("no-empty-pattern");
    }

    #[test]
    fn test_allows_empty_object_pattern_in_function_param_when_configured() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyPattern).with_options(|options| {
            options
                .correctness
                .no_empty_pattern_allow_object_patterns_as_parameters = true;
        });
        let result = test.lint_ast(
            "no_empty_pattern/test_allows_empty_object_pattern_in_function_param_when_configured.ds",
            r#"
function foo({}) {}
"#,
        );
        test.result(result).assert_no_lint("no-empty-pattern");
    }

    #[test]
    fn test_allows_non_empty_object_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyPattern);
        let result = test.lint_ast(
            "no_empty_pattern/test_allows_non_empty_object_pattern.ds",
            r#"
const { x } = obj
"#,
        );
        test.result(result).assert_no_lint("no-empty-pattern");
    }

    #[test]
    fn test_allows_non_empty_array_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyPattern);
        let result = test.lint_ast(
            "no_empty_pattern/test_allows_non_empty_array_pattern.ds",
            r#"
const [x] = arr
"#,
        );
        test.result(result).assert_no_lint("no-empty-pattern");
    }
}
