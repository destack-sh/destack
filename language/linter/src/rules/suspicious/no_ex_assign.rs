use crate::LintMeta;
use destack_ast::{self as ast, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    assign_pattern_is_unqualified_path_name, expression_is_direct_block_leading_expression,
    expression_is_direct_statement, expression_is_unqualified_path_name,
    expression_subtree_mentions_identifier_name,
};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Disallow reassigning exceptions in catch clauses.
    ///
    /// Reassigning the exception variable in a catch clause is almost always
    /// a mistake. It loses the original error information and makes debugging
    /// harder.
    #[lint(
        id = "no-ex-assign",
        code = "LU016",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoExAssign,
    "Disallow reassigning exceptions in catch clauses"
}

impl LintRule for NoExAssign {
    fn meta(&self) -> &'static LintMeta {
        NoExAssign::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Try {
                catch_pattern,
                catch_expression,
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // need both pattern and expression
            let (Some(pattern_id), Some(catch_expr_id)) = (catch_pattern, catch_expression) else {
                continue;
            };

            // extract the bound name
            let Some(catch_name) = get_pattern_binding_name(ctx, *pattern_id) else {
                continue;
            };

            // check for assignments in the catch body
            let assignment_expression_ids =
                collect_assignment_references_in_expression(ctx, *catch_expr_id, catch_name);
            for assignment_expression_id in assignment_expression_ids {
                report_ex_assign(
                    ctx,
                    meta,
                    *catch_expr_id,
                    assignment_expression_id,
                    catch_name,
                );
            }
        }
    }
}

/// Extract the binding name from a simple catch pattern.
fn get_pattern_binding_name(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<ast::StringId> {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        ast::Pattern::Binding { name, .. } => Some(*name),
        _ => None,
    }
}

/// Collect assignment expression ids that target one catch binding.
fn collect_assignment_references_in_expression(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    catch_name: ast::StringId,
) -> Vec<ast::LocalNodeId<ast::Expression>> {
    let mut visitor = CatchAssignmentCollector::new(catch_name);
    let expression = ctx.tree.get(expression_id);
    visitor.visit_expression(ctx.tree, expression_id, expression);
    visitor.assignment_ids
}

/// Report an exception reassignment diagnostic.
fn report_ex_assign(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
    catch_expression_id: ast::LocalNodeId<ast::Expression>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    catch_name: ast::StringId,
) {
    let severity = ctx.get_effective_severity(meta, expr_id);
    if !severity.is_enabled() {
        return;
    }

    let span = ctx.tree.get_span(expr_id);
    let mut diagnostic = LintDiagnostic::new(
        NO_EX_ASSIGN.id,
        NO_EX_ASSIGN.code,
        NO_EX_ASSIGN.category,
        severity,
        "do not reassign the exception variable",
        ctx.module.file_id,
        span,
    )
    .with_label("this reassignment loses the original error");
    if ctx.compute_fixes
        && let Some(fix) = no_ex_assign_fix(ctx, catch_expression_id, expr_id, catch_name)
    {
        diagnostic = diagnostic.with_fix(fix);
    }

    ctx.report(diagnostic);
}

/// Build one unsafe fix by replacing reassignment with a local alias binding.
fn no_ex_assign_fix(
    ctx: &LintAstContext<'_>,
    catch_expression_id: ast::LocalNodeId<ast::Expression>,
    assignment_expression_id: ast::LocalNodeId<ast::Expression>,
    catch_name: ast::StringId,
) -> Option<LintFix> {
    // keep standalone reassignment statements only
    let is_direct_statement =
        expression_is_direct_statement(ctx.tree, ctx.parents, assignment_expression_id);
    let is_direct_block_expression = expression_is_direct_block_leading_expression(
        ctx.tree,
        ctx.parents,
        assignment_expression_id,
    );
    if !is_direct_statement && !is_direct_block_expression {
        return None;
    }

    let assignment_expression = ctx.tree.get(assignment_expression_id);
    let ast::Expression::Assign { right, .. } = assignment_expression else {
        return None;
    };

    let right_text = ctx
        .get_span_text(ctx.tree.get_span(*right))
        .trim()
        .to_string();
    if right_text.is_empty() {
        return None;
    }
    if catch_name_is_used_after(
        ctx,
        catch_expression_id,
        catch_name,
        ctx.tree.get_span(assignment_expression_id).end,
    ) {
        return None;
    }

    let catch_name_text = ctx.strings.get(catch_name).to_string();
    let replacement_name = unique_catch_alias_name(ctx, catch_expression_id, &catch_name_text);
    let replacement_text = format!("let {replacement_name} = {right_text}");
    let edits = ctx
        .edit_builder()
        .replace(
            ctx.tree.get_span(assignment_expression_id),
            replacement_text,
        )
        .into_edits();
    Some(
        LintFix::r#unsafe("Introduce a new local binding instead of reassigning catch variable")
            .with_edits(edits),
    )
}

/// Return true when the catch binding name is referenced after a source offset.
fn catch_name_is_used_after(
    ctx: &LintAstContext<'_>,
    catch_expression_id: ast::LocalNodeId<ast::Expression>,
    catch_name: ast::StringId,
    offset: u32,
) -> bool {
    let catch_span = ctx.tree.get_span(catch_expression_id);
    for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
        if !expression_is_unqualified_path_name(ctx.tree, expression_id, catch_name) {
            continue;
        }

        let span = ctx.tree.get_span(expression_id);
        if span.start >= offset && span.start >= catch_span.start && span.end <= catch_span.end {
            return true;
        }
    }

    false
}

/// Build a catch-local unique alias for one catch variable.
fn unique_catch_alias_name(
    ctx: &LintAstContext<'_>,
    catch_expression_id: ast::LocalNodeId<ast::Expression>,
    catch_name: &str,
) -> String {
    let base_name = format!("{catch_name}Reassigned");
    let base_name_id = ctx.strings.intern(&base_name);
    if !expression_subtree_mentions_identifier_name(ctx.tree, catch_expression_id, base_name_id) {
        return base_name;
    }

    let mut suffix = 2_u32;
    loop {
        let candidate = format!("{base_name}{suffix}");
        let candidate_id = ctx.strings.intern(&candidate);
        if !expression_subtree_mentions_identifier_name(ctx.tree, catch_expression_id, candidate_id)
        {
            return candidate;
        }
        suffix += 1;
        if suffix > 1024 {
            return base_name;
        }
    }
}

/// Collect assignment expressions that reassign one catch binding.
struct CatchAssignmentCollector {
    /// The target catch variable name.
    catch_name: ast::StringId,
    /// Assignment expressions that target the catch name.
    assignment_ids: Vec<ast::LocalNodeId<ast::Expression>>,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl CatchAssignmentCollector {
    /// Build one collector for a catch binding.
    fn new(catch_name: ast::StringId) -> Self {
        Self {
            catch_name,
            assignment_ids: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }
}

impl NodeVisitor for CatchAssignmentCollector {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Expression>,
        expression: &ast::Expression,
    ) {
        // capture direct assignments to the catch binding
        if let ast::Expression::Assign { left, .. } = expression
            && assign_pattern_is_unqualified_path_name(tree, *left, self.catch_name)
        {
            self.assignment_ids.push(id);
        }

        // capture unary updates to the catch binding
        if let ast::Expression::Unary { operator, right } = expression
            && matches!(
                operator,
                ast::UnaryOperator::PreIncrement
                    | ast::UnaryOperator::PostIncrement
                    | ast::UnaryOperator::PreDecrement
                    | ast::UnaryOperator::PostDecrement
            )
            && expression_is_unqualified_path_name(tree, *right, self.catch_name)
        {
            self.assignment_ids.push(id);
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_exception_reassignment() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "no_ex_assign/test_flags_exception_reassignment.ds",
            r#"
try {
    riskyOperation();
} catch e {
    e = null;
}
"#,
        );
        test.result(result).assert_lint("no-ex-assign");
    }

    #[test]
    fn test_flags_exception_reassignment_with_new() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "no_ex_assign/test_flags_exception_reassignment_with_new.ds",
            r#"
try {
    riskyOperation();
} catch e {
    e = new Error("replaced");
}
"#,
        );
        test.result(result).assert_lint("no-ex-assign");
    }

    #[test]
    fn test_flags_exception_post_increment() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "no_ex_assign/test_flags_exception_post_increment.ds",
            r#"
try {
    riskyOperation();
} catch e {
    e++;
}
"#,
        );
        test.result(result).assert_lint("no-ex-assign");
    }

    #[test]
    fn test_flags_exception_pre_decrement() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "no_ex_assign/test_flags_exception_pre_decrement.ds",
            r#"
try {
    riskyOperation();
} catch e {
    --e;
}
"#,
        );
        test.result(result).assert_lint("no-ex-assign");
    }

    #[test]
    fn test_allows_catch_without_reassignment() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "no_ex_assign/test_allows_catch_without_reassignment.ds",
            r#"
try {
    riskyOperation();
} catch e {
    console.log(e);
    throw e;
}
"#,
        );
        test.result(result).assert_no_lint("no-ex-assign");
    }

    #[test]
    fn test_allows_different_variable_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "no_ex_assign/test_allows_different_variable_assignment.ds",
            r#"
try {
    riskyOperation();
} catch e {
    let message = e.message;
    message = "modified";
}
"#,
        );
        test.result(result).assert_no_lint("no-ex-assign");
    }

    #[test]
    fn test_fix_rewrites_exception_reassignment_to_local_alias() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let diagnostics = test.lint_ast(
            "no_ex_assign/test_fix_rewrites_exception_reassignment_to_local_alias.ds",
            r#"
try {
    riskyOperation();
} catch e {
    e = computeError();
}
"#,
        );
        let result = test.result(diagnostics);
        result
            .assert_lint("no-ex-assign")
            .assert_has_fix("no-ex-assign");
        let fixed = result.apply_fixes(None);
        assert_eq!(
            fixed.trim(),
            r#"
try {
    riskyOperation();
} catch (e) {
    let eReassigned = computeError();
}
"#
            .trim()
        );
    }

    #[test]
    fn test_no_fix_when_reassignment_is_used_as_expression() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "no_ex_assign/test_no_fix_when_reassignment_is_used_as_expression.ds",
            r#"
try {
    riskyOperation();
} catch e {
    ((e = fallbackError()))
}
"#,
        );
        test.result(result)
            .assert_lint("no-ex-assign")
            .assert_has_no_fix("no-ex-assign");
    }

    #[test]
    fn test_detects_reassignment_nested_in_loop_inside_catch() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "no_ex_assign/test_detects_reassignment_nested_in_loop_inside_catch.ds",
            r#"
try {
    riskyOperation();
} catch e {
    while (needsRetry()) {
        e = nextError();
        break;
    }
}
"#,
        );
        test.result(result).assert_lint("no-ex-assign");
    }

    #[test]
    fn test_no_fix_when_catch_binding_is_used_later() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "no_ex_assign/test_no_fix_when_catch_binding_is_used_later.ds",
            r#"
try {
    riskyOperation();
} catch e {
    e = fallbackError();
    log(e);
}
"#,
        );
        test.result(result)
            .assert_lint("no-ex-assign")
            .assert_has_no_fix("no-ex-assign");
    }

    #[test]
    fn test_fix_uses_suffix_when_alias_name_exists() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let diagnostics = test.lint_ast(
            "no_ex_assign/test_fix_uses_suffix_when_alias_name_exists.ds",
            r#"
try {
    riskyOperation();
} catch e {
    let eReassigned = currentError();
    e = computeError();
}
"#,
        );
        test.result(diagnostics)
            .assert_lint("no-ex-assign")
            .assert_unsafe_fixed(
                r#"
try {
    riskyOperation();
} catch (e) {
    let eReassigned = currentError();
    let eReassigned2 = computeError();
}
"#,
            );
    }

    #[test]
    fn test_fix_keeps_base_alias_name_when_outer_name_is_unrelated() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let diagnostics = test.lint_ast(
            "no_ex_assign/test_fix_keeps_base_alias_name_when_outer_name_is_unrelated.ds",
            r#"
let eReassigned = previousError();

try {
    riskyOperation();
} catch e {
    e = computeError();
}
"#,
        );
        test.result(diagnostics)
            .assert_lint("no-ex-assign")
            .assert_unsafe_fixed(
                r#"
let eReassigned = previousError();

try {
    riskyOperation();
} catch (e) {
    let eReassigned = computeError();
}
"#,
            );
    }
}
