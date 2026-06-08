use crate::LintMeta;
use destack_core::StringId;
use destack_dir::{
    self as dir, AssignOperator, Block, Declarator, Expression, IfCondition, IfForm, LetKind,
    LocalNodeId, Pattern, Tree,
};
use destack_repository::LintSeverity;

use crate::rules::common::{assign_pattern_expression, expression_path_segments};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer expression-based if over statement-based pattern.
    ///
    /// Detects patterns where a variable is declared without initialization
    /// and then immediately assigned in both branches of an if statement.
    /// Such patterns should use expression-based constructs instead.
    #[lint(
        id = "prefer-expression",
        code = "LY037",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferExpression,
    "Prefer expression-based if over statement pattern"
}

/// One matched let-then-if assignment pattern.
#[derive(Clone, Copy)]
struct ExpressionPatternCandidate {
    /// The original let expression id from the block.
    let_expression_id: LocalNodeId<Expression>,
    /// The original if expression id from the block.
    if_expression_id: LocalNodeId<Expression>,
    /// The declaration kind (`let` or `const`).
    declaration_kind: LetKind,
    /// The declarator id for preserving annotations.
    declarator_id: LocalNodeId<Declarator>,
    /// The if condition expression id.
    condition_expression_id: LocalNodeId<Expression>,
    /// The then-branch assigned value expression.
    then_value_expression_id: LocalNodeId<Expression>,
    /// The else-branch assigned value expression.
    else_value_expression_id: LocalNodeId<Expression>,
}

impl LintRule for PreferExpression {
    fn meta(&self) -> &'static LintMeta {
        PreferExpression::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for root_id in ctx.roots.clone() {
            check_expression(ctx, meta, ctx.dir.tree(), root_id);
        }
    }
}

/// Recursively check expressions for the pattern.
fn check_expression(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    tree: &Tree,
    expression_id: LocalNodeId<Expression>,
) {
    let expression = tree.get(expression_id);

    // check blocks for the pattern
    if let Expression::Block(block_id) = expression {
        check_block(ctx, meta, tree, *block_id);
    }

    // recursively check nested expressions
    match expression {
        Expression::Block(block_id) => {
            let block = tree.get(*block_id);
            for child_expression_id in block.iter_expressions() {
                check_expression(ctx, meta, tree, child_expression_id);
            }
        }
        Expression::If {
            then_expression,
            else_expression,
            ..
        } => {
            check_expression(ctx, meta, tree, *then_expression);
            if let Some(else_expression_id) = else_expression {
                check_expression(ctx, meta, tree, *else_expression_id);
            }
        }
        Expression::Declaration(declaration_id) => {
            let declaration = tree.get(*declaration_id);
            if let dir::Declaration::Function(declaration) = declaration
                && let Some(body_expression_id) = declaration.body
            {
                check_expression(ctx, meta, tree, body_expression_id);
            }
        }
        _ => {}
    }
}

/// Check a block for the uninitialized-let-then-if pattern.
fn check_block(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    tree: &Tree,
    block_id: LocalNodeId<Block>,
) {
    let block = tree.get(block_id);
    let expressions = block.iter_expressions().collect::<Vec<_>>();

    // keep adjacent let + if pairs
    for window_index in 0..expressions.len().saturating_sub(1) {
        let let_expression_id = expressions[window_index];
        let if_expression_id = expressions[window_index + 1];

        let Some(candidate) =
            expression_pattern_candidate(tree, let_expression_id, if_expression_id)
        else {
            continue;
        };

        let severity = ctx.get_effective_severity(meta, candidate.let_expression_id);
        if !severity.is_enabled() {
            continue;
        }

        let mut diagnostic = LintReport::new(
            PREFER_EXPRESSION.id,
            PREFER_EXPRESSION.code,
            PREFER_EXPRESSION.category,
            severity,
            "prefer expression-based if over statement pattern",
            tree.get_span(candidate.let_expression_id),
        )
        .label("use `const x = if (condition) { a } else { b }` instead");
        if ctx.compute_fixes
            && let Some(fix) = prefer_expression_fix(ctx, candidate)
        {
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// Build one expression-pattern candidate from adjacent expressions.
fn expression_pattern_candidate(
    tree: &Tree,
    let_expression_id: LocalNodeId<Expression>,
    if_expression_id: LocalNodeId<Expression>,
) -> Option<ExpressionPatternCandidate> {
    let (declaration_kind, declarator_id, variable_name) =
        uninitialized_let_declarator(tree, let_expression_id)?;
    let (condition_expression_id, then_value_expression_id, else_value_expression_id) =
        if_assignment_pattern(tree, if_expression_id, variable_name)?;

    Some(ExpressionPatternCandidate {
        let_expression_id,
        if_expression_id,
        declaration_kind,
        declarator_id,
        condition_expression_id,
        then_value_expression_id,
        else_value_expression_id,
    })
}

/// Return the declaration kind, declarator, and name for an uninitialized let binding.
fn uninitialized_let_declarator(
    tree: &Tree,
    expression_id: LocalNodeId<Expression>,
) -> Option<(LetKind, LocalNodeId<Declarator>, StringId)> {
    let expression_id = unwrap_statement_expression(tree, expression_id);
    let expression = tree.get(expression_id);
    let Expression::Let {
        kind, declarators, ..
    } = expression
    else {
        return None;
    };

    // keep one declarator without initializer
    if declarators.len() != 1 {
        return None;
    }

    let declarator_id = declarators[0];
    let declarator = tree.get(declarator_id);
    if declarator.value.is_some() {
        return None;
    }

    let variable_name = simple_binding_name(tree, declarator.pattern)?;
    Some((*kind, declarator_id, variable_name))
}

/// Return assignment details for `if` patterns that assign both branches.
fn if_assignment_pattern(
    tree: &Tree,
    expression_id: LocalNodeId<Expression>,
    variable_name: StringId,
) -> Option<(
    LocalNodeId<Expression>,
    LocalNodeId<Expression>,
    LocalNodeId<Expression>,
)> {
    let expression_id = unwrap_statement_expression(tree, expression_id);
    let expression = tree.get(expression_id);
    let Expression::If {
        form: IfForm::If,
        condition,
        then_expression,
        else_expression: Some(else_expression),
    } = expression
    else {
        return None;
    };

    // keep explicit expression conditions
    let IfCondition::Expression { condition } = condition else {
        return None;
    };

    // keep one assignment in each branch
    let then_value_expression_id = branch_assigned_value(tree, *then_expression, variable_name)?;
    let else_value_expression_id = branch_assigned_value(tree, *else_expression, variable_name)?;

    Some((
        *condition,
        then_value_expression_id,
        else_value_expression_id,
    ))
}

/// Return the assigned value in a branch when it is one simple assignment.
fn branch_assigned_value(
    tree: &Tree,
    expression_id: LocalNodeId<Expression>,
    variable_name: StringId,
) -> Option<LocalNodeId<Expression>> {
    let expression = tree.get(expression_id);

    // keep one expression blocks only
    let assignment_expression_id = if let Expression::Block(block_id) = expression {
        let block = tree.get(*block_id);
        if block.len() != 1 {
            return None;
        }
        unwrap_statement_expression(tree, block.first_expression()?)
    } else {
        unwrap_statement_expression(tree, expression_id)
    };

    let assignment_expression = tree.get(assignment_expression_id);
    let Expression::Assign {
        left,
        operator: AssignOperator::Assign,
        right,
    } = assignment_expression
    else {
        return None;
    };

    // keep plain `name = value` assignments
    let left_expression_id = assign_pattern_expression(tree, *left)?;
    if !is_path_with_name(tree, left_expression_id, variable_name) {
        return None;
    }

    Some(*right)
}

/// Build a safe rewrite from `let x; if (...) { x = a } else { x = b }` to expression form.
fn prefer_expression_fix(
    ctx: &LintModuleContext<'_>,
    candidate: ExpressionPatternCandidate,
) -> Option<LintFix> {
    let declaration_kind = match candidate.declaration_kind {
        LetKind::Let => "let",
        LetKind::Const => "const",
    };

    let declarator_text = ctx.get_span_text(ctx.dir.get_span(candidate.declarator_id));
    let condition_text = ctx.get_span_text(ctx.dir.get_span(candidate.condition_expression_id));
    let then_value_text = ctx.get_span_text(ctx.dir.get_span(candidate.then_value_expression_id));
    let else_value_text = ctx.get_span_text(ctx.dir.get_span(candidate.else_value_expression_id));

    let replacement = format!(
        "{declaration_kind} {declarator_text} = if {condition_text} {{ {then_value_text} }} else {{ {else_value_text} }};"
    );

    let edits = ctx
        .edit_builder()
        .replace(ctx.dir.get_span(candidate.let_expression_id), replacement)
        .replace(ctx.dir.get_span(candidate.if_expression_id), "")
        .into_edits();

    Some(LintFix::safe("Rewrite adjacent let-if assignment to expression form").with_edits(edits))
}

/// Return the simple binding name from one pattern.
fn simple_binding_name(tree: &Tree, pattern_id: LocalNodeId<Pattern>) -> Option<StringId> {
    let pattern = tree.get(pattern_id);
    match pattern {
        Pattern::Binding {
            name,
            pattern: None,
            ..
        } => Some(*name),
        _ => None,
    }
}

/// Return true when one expression is a simple path to the given name.
fn is_path_with_name(tree: &Tree, expression_id: LocalNodeId<Expression>, name: StringId) -> bool {
    expression_path_segments(tree, expression_id)
        .is_some_and(|path_segments| path_segments.as_slice() == [name])
}

/// Unwrap one statement wrapper when present.
fn unwrap_statement_expression(
    tree: &Tree,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let _ = tree;
    expression_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_uninitialized_let_with_if_assignment() {
        let test = TestProgram::for_rule_without_prelude(PreferExpression);
        let result = test.lint(
            "prefer_expression/test_detects_uninitialized_let_with_if_assignment.ds",
            r#"
function foo(condition: boolean) {
    let x;
    if (condition) {
        x = 1;
    } else {
        x = 2;
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-expression")
            .assert_safe_fixed(
                r#"
function foo(condition: boolean) {
    let x = if (condition) { 1 } else { 2 };
}
"#,
            );
    }

    #[test]
    fn test_fix_preserves_type_annotation() {
        let test = TestProgram::for_rule_without_prelude(PreferExpression);
        let result = test.lint(
            "prefer_expression/test_fix_preserves_type_annotation.ds",
            r#"
function foo(condition: boolean) {
    let x: int32;
    if (condition) {
        x = 1;
    } else {
        x = 2;
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-expression")
            .assert_safe_fixed(
                r#"
function foo(condition: boolean) {
    let x: int32 = if (condition) { 1 } else { 2 };
}
"#,
            );
    }

    #[test]
    fn test_allows_initialized_let() {
        let test = TestProgram::for_rule_without_prelude(PreferExpression);
        let result = test.lint(
            "prefer_expression/test_allows_initialized_let.ds",
            r#"
function foo(condition: boolean) {
    let x = 0;
    if (condition) {
        x = 1;
    } else {
        x = 2;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_expression_based_if() {
        let test = TestProgram::for_rule_without_prelude(PreferExpression);
        let result = test.lint(
            "prefer_expression/test_allows_expression_based_if.ds",
            r#"
function foo(condition: boolean) {
    const x = if (condition) { 1 } else { 2 };
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_if_without_else() {
        let test = TestProgram::for_rule_without_prelude(PreferExpression);
        let result = test.lint(
            "prefer_expression/test_allows_if_without_else.ds",
            r#"
function foo(condition: boolean) {
    let x;
    if (condition) {
        x = 1;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_if_not_assigning_to_same_variable() {
        let test = TestProgram::for_rule_without_prelude(PreferExpression);
        let result = test.lint(
            "prefer_expression/test_allows_if_not_assigning_to_same_variable.ds",
            r#"
function foo(condition: boolean) {
    let x;
    let y;
    if (condition) {
        x = 1;
    } else {
        y = 2;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_non_adjacent_statements() {
        let test = TestProgram::for_rule_without_prelude(PreferExpression);
        let result = test.lint(
            "prefer_expression/test_allows_non_adjacent_statements.ds",
            r#"
function foo(condition: boolean) {
    let x;
    console.log("something");
    if (condition) {
        x = 1;
    } else {
        x = 2;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_if_expression_without_assignment_branches() {
        let test = TestProgram::for_rule_without_prelude(PreferExpression);
        let result = test.lint(
            "prefer_expression/test_allows_if_expression_without_assignment_branches.ds",
            r#"
function foo(condition: boolean): int32 {
    let x;
    if (condition) {
        x = 1;
    } else {
        return 2;
    }

    return x;
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_branch_with_multiple_statements() {
        let test = TestProgram::for_rule_without_prelude(PreferExpression);
        let result = test.lint(
            "prefer_expression/test_allows_branch_with_multiple_statements.ds",
            r#"
function foo(condition: boolean) {
    let x;
    if (condition) {
        console.log("yes");
        x = 1;
    } else {
        x = 2;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }
}
