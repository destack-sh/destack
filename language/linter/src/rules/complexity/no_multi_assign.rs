use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    assign_pattern_expression, expression_outer_parenthesized_source_form,
    expression_path_segments, expression_statement_ancestor,
    expression_unwrap_parenthesized_source_form,
};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow chained assignment expressions.
    ///
    /// Chained assignments like `a = b = c` can be confusing about which variables are being modified.
    /// Write each assignment on its own line for clarity.
    #[lint(
        id = "no-multi-assign",
        code = "LX019",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoMultiAssign,
    "Disallow chained assignments"
}

impl LintRule for NoMultiAssign {
    fn meta(&self) -> &'static LintMeta {
        NoMultiAssign::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata and option policy
        let meta = self.meta();
        let ignore_non_declaration = ctx
            .options()
            .complexity
            .no_multi_assign_ignore_non_declaration;

        // inspect each assignment expression for chained-assignment contexts
        for expression_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Assign { .. } = ctx.dir.get(expression_id) else {
                continue;
            };

            // keep only source-equivalent report contexts
            if !assignment_should_report(ctx, expression_id, ignore_non_declaration) {
                continue;
            }

            // resolve effective severity for this assignment expression
            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // build the base diagnostic for this chain node
            let span = ctx.dir.get_span(expression_id);
            let mut diagnostic = LintReport::new(
                NO_MULTI_ASSIGN.id,
                NO_MULTI_ASSIGN.code,
                NO_MULTI_ASSIGN.category,
                severity,
                "chained assignment expression",
                span,
            )
            .label("write each assignment on its own line");

            // compute fixes only when requested by the runner
            if ctx.compute_fixes
                && let Some(fix) = no_multi_assign_fix(ctx, expression_id)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when one assignment should be reported by no-multi-assign.
fn assignment_should_report(
    ctx: &LintModuleContext<'_>,
    assignment_expression_id: dir::LocalNodeId<dir::Expression>,
    ignore_non_declaration: bool,
) -> bool {
    // always report declaration initializer assignments
    if assignment_is_declaration_initializer(ctx, assignment_expression_id) {
        return true;
    }

    // optionally ignore non-declaration chains
    if ignore_non_declaration {
        return false;
    }

    // report non-declaration chains on outer assignment nodes
    assignment_has_assignment_right(ctx, assignment_expression_id)
}

/// Return true when one assignment is in a declaration initializer slot.
fn assignment_is_declaration_initializer(
    ctx: &LintModuleContext<'_>,
    assignment_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // lift assignment through parenthesized wrappers for parent checks
    let wrapped_expression_id =
        expression_outer_parenthesized_source_form(ctx.dir.tree(), assignment_expression_id);

    // require one concrete parent node
    let Some(parent_id) = ctx.dir.get_parent_id(wrapped_expression_id.id) else {
        return false;
    };

    // match declarator initializers
    if ctx.dir.get_node_type(parent_id) == dir::NodeType::Declarator {
        let declarator = ctx
            .dir
            .tree()
            .get(dir::LocalNodeId::<dir::Declarator>::new(parent_id));
        if matches!(declarator, dir::Declarator { value: Some(value_id), .. } if *value_id == wrapped_expression_id)
        {
            return true;
        }
    }

    // match class or interface field defaults
    if ctx.dir.get_node_type(parent_id) == dir::NodeType::Member {
        let member = ctx
            .dir
            .tree()
            .get(dir::LocalNodeId::<dir::Member>::new(parent_id));
        if matches!(member, dir::Member::Field { default: Some(default_id), .. } if *default_id == wrapped_expression_id)
        {
            return true;
        }
    }

    // match object or type-literal field defaults
    if ctx.dir.get_node_type(parent_id) == dir::NodeType::Property {
        let property = ctx
            .dir
            .tree()
            .get(dir::LocalNodeId::<dir::Property>::new(parent_id));
        if matches!(property, dir::Property::Field { value, .. } if *value == wrapped_expression_id)
        {
            return true;
        }
    }

    false
}

/// Return true when one assignment has an assignment expression on the right.
fn assignment_has_assignment_right(
    ctx: &LintModuleContext<'_>,
    assignment_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // normalize the assignment expression before right-side checks
    let assignment_expression_id =
        expression_unwrap_parenthesized_source_form(ctx.dir.tree(), assignment_expression_id);
    let assignment_expression = ctx.dir.get(assignment_expression_id);
    let dir::Expression::Assign { right, .. } = assignment_expression else {
        return false;
    };

    // keep only right sides that resolve to another assignment expression
    let right_expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *right);
    let right_expression = ctx.dir.get(right_expression_id);
    matches!(right_expression, dir::Expression::Assign { .. })
}

/// Build a safe fix for one chained assignment statement with simple paths.
fn no_multi_assign_fix(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    // keep statement scoped assignments only
    let parent_expression_id = expression_statement_ancestor(ctx.dir.tree(), expression_id)?;

    // collect chained assignments and final rhs
    let mut left_ids = Vec::new();
    let final_rhs_id = collect_assignment_chain(ctx, expression_id, &mut left_ids)?;
    if left_ids.len() < 2 {
        return None;
    }

    // collect lhs texts and keep simple paths only
    let mut left_texts = Vec::new();
    for left_id in left_ids {
        expression_path_segments(ctx.dir.tree(), left_id)?;

        let left_span = ctx.dir.get_span(left_id);
        left_texts.push(ctx.get_span_text(left_span).to_string());
    }

    let final_rhs_span = ctx.dir.get_span(final_rhs_id);
    let final_rhs_text = ctx.get_span_text(final_rhs_span).to_string();

    // build sequential assignments from inner to outer
    let mut rewrites = Vec::new();
    let mut previous_value = final_rhs_text;
    for left_text in left_texts.into_iter().rev() {
        rewrites.push(format!("{left_text} = {previous_value};"));
        previous_value = left_text;
    }

    let replacement = rewrites.join("\n");
    let statement_span = ctx.dir.get_span(parent_expression_id);
    let edits = ctx
        .edit_builder()
        .replace(statement_span, replacement)
        .into_edits();
    Some(LintFix::safe("Split chained assignment into sequential assignments").with_edits(edits))
}

/// Collect left sides for a chained assignment and return the final rhs expression.
fn collect_assignment_chain(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    left_ids: &mut Vec<dir::LocalNodeId<dir::Expression>>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);
    let dir::Expression::Assign { left, right, .. } = expression else {
        return Some(expression_id);
    };

    let left_expression_id = assign_pattern_expression(ctx.dir.tree(), *left)?;
    left_ids.push(left_expression_id);

    collect_assignment_chain(ctx, *right, left_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_chained_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_detects_chained_assignment.ds",
            r#"
let a: int32;
let b: int32;
let c: int32;
a = (b = (c = 1));
"#,
        );
        test.result(result).assert_lint("no-multi-assign");
    }

    #[test]
    fn test_detects_simple_chain() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_detects_simple_chain.ds",
            r#"
let a: int32;
let b: int32;
a = (b = 1);
"#,
        );
        test.result(result).assert_lint("no-multi-assign");
    }

    #[test]
    fn test_detects_parenthesized_chain() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_detects_parenthesized_chain.ds",
            r#"
let a: int32;
let b: int32;
a = (b = 1);
"#,
        );
        test.result(result).assert_lint("no-multi-assign");
    }

    #[test]
    fn test_allows_separate_assignments() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_allows_separate_assignments.ds",
            r#"
let a: int32;
let b: int32;
a = 1;
b = 1;
"#,
        );
        test.result(result).assert_no_lint("no-multi-assign");
    }

    #[test]
    fn test_allows_assignment_in_declaration() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_allows_assignment_in_declaration.ds",
            r#"
let a = 1;
let b = 2;
"#,
        );
        test.result(result).assert_no_lint("no-multi-assign");
    }

    #[test]
    fn test_allows_compound_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_allows_compound_assignment.ds",
            r#"
let a = 1;
a += 2;
"#,
        );
        test.result(result).assert_no_lint("no-multi-assign");
    }

    #[test]
    fn test_fix_splits_simple_chained_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_fix_splits_simple_chained_assignment.ds",
            r#"
let a: int32;
let b: int32;
a = (b = 1);
"#,
        );
        test.result(result)
            .assert_lint("no-multi-assign")
            .assert_safe_fixed(
                r#"
let a: int32;
let b: int32;
b = 1;
a = b;
"#,
            );
    }

    #[test]
    fn test_fix_splits_nested_chained_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_fix_splits_nested_chained_assignment.ds",
            r#"
let a: int32;
let b: int32;
let c: int32;
a = (b = (c = 1));
"#,
        );
        test.result(result)
            .assert_lint("no-multi-assign")
            .assert_safe_fixed(
                r#"
let a: int32;
let b: int32;
let c: int32;
c = 1;
b = c;
a = b;
"#,
            );
    }

    #[test]
    fn test_no_fix_for_chained_assignment_used_as_value() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_no_fix_for_chained_assignment_used_as_value.ds",
            r#"
let a: int32;
let b: int32;
let x = (a = (b = 1));
"#,
        );
        test.result(result)
            .assert_lint("no-multi-assign")
            .assert_has_no_fix("no-multi-assign");
    }

    #[test]
    fn test_detects_assignment_in_declaration_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint(
            "no_multi_assign/test_detects_assignment_in_declaration_initializer.ds",
            r#"
let a: int32;
let x = (a = 1);
"#,
        );
        test.result(result).assert_lint("no-multi-assign");
    }

    #[test]
    fn test_ignore_non_declaration_option_ignores_assignment_chain() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign).with_options(|options| {
            options.complexity.no_multi_assign_ignore_non_declaration = true;
        });
        let result = test.lint(
            "no_multi_assign/test_ignore_non_declaration_option_ignores_assignment_chain.ds",
            r#"
let a: int32;
let b: int32;
a = (b = 1);
"#,
        );
        test.result(result).assert_no_lint("no-multi-assign");
    }

    #[test]
    fn test_ignore_non_declaration_option_still_reports_declaration_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign).with_options(|options| {
            options.complexity.no_multi_assign_ignore_non_declaration = true;
        });
        let result = test.lint(
            "no_multi_assign/test_ignore_non_declaration_option_still_reports_declaration_initializer.ds",
            r#"
let a: int32;
let x = (a = 1);
"#,
        );
        test.result(result).assert_lint("no-multi-assign");
    }
}
