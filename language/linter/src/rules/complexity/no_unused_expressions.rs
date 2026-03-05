use destack_ast::{self as ast, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_has_side_effects, expression_unwrap_parenthesized_syntax};
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow statement expressions that have no effect.
    ///
    /// Expressions that are evaluated and discarded are often mistakes or leftovers from refactors.
    #[lint(
        id = "no-unused-expressions",
        code = "LX022",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoUnusedExpressions,
    "Disallow expressions without effect"
}

impl LintRule for NoUnusedExpressions {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnusedExpressions::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // inspect module root statement candidates
        for statement_expression_id in ctx.roots {
            check_statement_candidate(ctx, meta, *statement_expression_id);
        }

        // inspect statement candidates inside each block body
        for block_id in ctx.tree.iter_nodes::<ast::Block>() {
            let block = ctx.tree.get(block_id);
            for statement_expression_id in &block.expressions {
                check_statement_candidate(ctx, meta, *statement_expression_id);
            }
        }
    }
}

/// Check one root or block expression as a statement candidate.
fn check_statement_candidate(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    statement_expression_id: ast::LocalNodeId<ast::Expression>,
) {
    // resolve statement wrapper semantics
    let Some(inner_expression_id) = statement_expression_inner_id(ctx, statement_expression_id)
    else {
        return;
    };

    // keep directive prologues out of reports
    if expression_statement_is_directive(ctx, statement_expression_id, inner_expression_id) {
        return;
    }

    // skip expressions that are valid in statement position
    if !expression_is_disallowed_in_statement(ctx, inner_expression_id) {
        return;
    }

    // resolve effective severity for this statement expression
    let severity = ctx.get_effective_severity(meta, statement_expression_id);
    if !severity.is_enabled() {
        return;
    }

    // report one unused expression diagnostic
    ctx.report(
        LintDiagnostic::new(
            NO_UNUSED_EXPRESSIONS.id,
            NO_UNUSED_EXPRESSIONS.code,
            NO_UNUSED_EXPRESSIONS.category,
            severity,
            "expression statement has no effect",
            ctx.module.file_id,
            ctx.tree.get_span(statement_expression_id),
        )
        .with_label("expected an assignment or call in statement position"),
    );
}

/// Return one inner expression id when an expression is statement like.
fn statement_expression_inner_id(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    // unwrap explicit statement wrappers
    if let ast::Expression::Statement(inner_expression_id) = ctx.tree.get(expression_id) {
        return Some(*inner_expression_id);
    }

    // keep only implicit expression statements at root and block level
    if ctx.tree.get(expression_id).is_top_level_statement() {
        return None;
    }

    Some(expression_id)
}

/// Return true when one statement expression is in a directive prologue.
fn expression_statement_is_directive(
    ctx: &LintModuleAstContext<'_>,
    statement_expression_id: ast::LocalNodeId<ast::Expression>,
    inner_expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // directives are string literals only
    let inner_expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, inner_expression_id);
    let inner_expression = ctx.tree.get(inner_expression_id);
    if !matches!(
        inner_expression,
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_))
    ) {
        return false;
    }

    // allow directives at module scope
    if let Some(statement_index) = expression_index_in_slice(ctx.roots, statement_expression_id) {
        return expression_prefix_is_all_directives(ctx, &ctx.roots[..statement_index]);
    }

    // allow directives in function and method block bodies
    let Some((block_id, statement_index)) =
        enclosing_block_statement_index(ctx, statement_expression_id)
    else {
        return false;
    };
    if !block_is_directive_capable(ctx, block_id) {
        return false;
    }

    let block = ctx.tree.get(block_id);
    expression_prefix_is_all_directives(ctx, &block.expressions[..statement_index])
}

/// Return true when all statement expressions in one prefix are directive literals.
fn expression_prefix_is_all_directives(
    ctx: &LintModuleAstContext<'_>,
    statement_prefix: &[ast::LocalNodeId<ast::Expression>],
) -> bool {
    // require every earlier statement to be a string literal statement
    statement_prefix.iter().all(|expression_id| {
        let Some(inner_expression_id) = statement_expression_inner_id(ctx, *expression_id) else {
            return false;
        };
        let inner_expression_id =
            expression_unwrap_parenthesized_syntax(ctx.tree, inner_expression_id);
        matches!(
            ctx.tree.get(inner_expression_id),
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_))
        )
    })
}

/// Return block id and statement index when one statement belongs to one block body.
fn enclosing_block_statement_index(
    ctx: &LintModuleAstContext<'_>,
    statement_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<(ast::LocalNodeId<ast::Block>, usize)> {
    // require a block parent for statement expressions
    let block_id = ctx.parents.get(statement_expression_id)?;
    if ctx.tree.get_node_type(block_id) != ast::NodeType::Block {
        return None;
    }
    let block_id = ast::LocalNodeId::<ast::Block>::new(block_id);
    let block = ctx.tree.get(block_id);
    let statement_index = expression_index_in_slice(&block.expressions, statement_expression_id)?;

    Some((block_id, statement_index))
}

/// Return true when one block can host a directive prologue.
fn block_is_directive_capable(
    ctx: &LintModuleAstContext<'_>,
    block_id: ast::LocalNodeId<ast::Block>,
) -> bool {
    // resolve the parent expression for this block
    let Some(block_expression_raw_id) = ctx.parents.get(block_id) else {
        return false;
    };
    if ctx.tree.get_node_type(block_expression_raw_id) != ast::NodeType::Expression {
        return false;
    }
    let block_expression_id = ast::LocalNodeId::<ast::Expression>::new(block_expression_raw_id);
    let block_expression = ctx.tree.get(block_expression_id);
    if !matches!(block_expression, ast::Expression::Block(inner) if *inner == block_id) {
        return false;
    }

    // resolve owner node for this block expression
    let Some(owner_raw_id) = ctx.parents.get(block_expression_id) else {
        return false;
    };
    let owner_type = ctx.tree.get_node_type(owner_raw_id);

    // allow function declaration bodies
    if owner_type == ast::NodeType::Declaration {
        let owner_id = ast::LocalNodeId::<ast::Declaration>::new(owner_raw_id);
        let owner = ctx.tree.get(owner_id);
        return matches!(
            owner,
            ast::Declaration::Function {
                body: Some(body_expression_id),
                ..
            } if *body_expression_id == block_expression_id
        );
    }

    // allow class and interface method bodies
    if owner_type == ast::NodeType::Member {
        let owner_id = ast::LocalNodeId::<ast::Member>::new(owner_raw_id);
        let owner = ctx.tree.get(owner_id);
        return matches!(
            owner,
            ast::Member::Method {
                body: Some(body_expression_id),
                ..
            } if *body_expression_id == block_expression_id
        );
    }

    // allow object method bodies
    if owner_type == ast::NodeType::Property {
        let owner_id = ast::LocalNodeId::<ast::Property>::new(owner_raw_id);
        let owner = ctx.tree.get(owner_id);
        return matches!(
            owner,
            ast::Property::Method {
                body: Some(body_expression_id),
                ..
            } if *body_expression_id == block_expression_id
        );
    }

    false
}

/// Return true when one expression shape is disallowed as a statement expression.
fn expression_is_disallowed_in_statement(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // normalize parenthesized wrappers first
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);

    // recursion wrappers around expression statements
    if let ast::Expression::Statement(inner_expression_id) = expression {
        return expression_is_disallowed_in_statement(ctx, *inner_expression_id);
    }
    if let ast::Expression::Must { left, .. } | ast::Expression::Maybe { left, .. } = expression {
        return expression_is_disallowed_in_statement(ctx, *left);
    }
    if let ast::Expression::Instantiation { left, .. } = expression {
        return expression_is_disallowed_in_statement(ctx, *left);
    }
    if let ast::Expression::TypeUnary { right, .. } = expression {
        return expression_is_disallowed_in_statement(ctx, *right);
    }
    if let ast::Expression::ReferenceOf { right, .. }
    | ast::Expression::ValueOf { right, .. }
    | ast::Expression::PointerOf { right, .. } = expression
    {
        return expression_is_disallowed_in_statement(ctx, *right);
    }

    // eslint semantics: delete and void are allowed statements
    if let ast::Expression::Delete { .. } = expression {
        return false;
    }
    if let ast::Expression::Unary { operator, .. } = expression {
        return !matches!(
            operator,
            UnaryOperator::PreIncrement
                | UnaryOperator::PostIncrement
                | UnaryOperator::PreDecrement
                | UnaryOperator::PostDecrement
                | UnaryOperator::Void
        );
    }

    // known side effectful expression statements
    if matches!(
        expression,
        ast::Expression::Call { .. }
            | ast::Expression::Assign { .. }
            | ast::Expression::New { .. }
            | ast::Expression::Await { .. }
            | ast::Expression::AwaitMaybe { .. }
            | ast::Expression::Yield { .. }
            | ast::Expression::Import { .. }
            | ast::Expression::Export { .. }
            | ast::Expression::ExportNamespace { .. }
            | ast::Expression::Let { .. }
            | ast::Expression::Using { .. }
            | ast::Expression::Declaration(_)
            | ast::Expression::Return { .. }
            | ast::Expression::Break { .. }
            | ast::Expression::Continue { .. }
            | ast::Expression::Throw { .. }
            | ast::Expression::If { .. }
            | ast::Expression::For { .. }
            | ast::Expression::ForEach { .. }
            | ast::Expression::While { .. }
            | ast::Expression::Loop { .. }
            | ast::Expression::Match { .. }
            | ast::Expression::Try { .. }
            | ast::Expression::Block(_)
            | ast::Expression::Labelled { .. }
            | ast::Expression::Debugger
            | ast::Expression::Error
            | ast::Expression::Stub
    ) {
        return false;
    }

    // known disallowed pure expression statement families
    if matches!(
        expression,
        ast::Expression::Path { .. }
            | ast::Expression::Member { .. }
            | ast::Expression::PrivateMember { .. }
            | ast::Expression::Index { .. }
            | ast::Expression::Binary { .. }
            | ast::Expression::TypeBinary { .. }
            | ast::Expression::TypeConditional { .. }
            | ast::Expression::TypeMapped { .. }
            | ast::Expression::TypeIndex { .. }
            | ast::Expression::TypeTemplateLiteral { .. }
            | ast::Expression::TypeImport { .. }
            | ast::Expression::TypeInfer { .. }
            | ast::Expression::TypePredicate { .. }
            | ast::Expression::ScalarLiteral(_)
            | ast::Expression::TypeLiteral(_)
            | ast::Expression::ArrayExpression { .. }
            | ast::Expression::TupleExpression { .. }
            | ast::Expression::ObjectExpression { .. }
            | ast::Expression::TreeExpression { .. }
            | ast::Expression::SequenceExpression { .. }
            | ast::Expression::TemplateExpression { .. }
            | ast::Expression::TaggedTemplateExpression { .. }
            | ast::Expression::PrivateIdentifier { .. }
            | ast::Expression::This
            | ast::Expression::Super
    ) {
        return true;
    }

    // fallback for unknown shapes: flag when side effects are absent
    !expression_has_side_effects(ctx, expression_id)
}

/// Return index of one expression id in one expression slice.
fn expression_index_in_slice(
    expressions: &[ast::LocalNodeId<ast::Expression>],
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<usize> {
    expressions
        .iter()
        .position(|current_expression_id| *current_expression_id == expression_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_unused_literal() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_detects_unused_literal.ds",
            r#"
5;
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_detects_unused_literal_without_semicolon() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_detects_unused_literal_without_semicolon.ds",
            r#"
5
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_detects_unused_binary_expression() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_detects_unused_binary_expression.ds",
            r#"
x + 1;
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_detects_unused_template_expression() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_detects_unused_template_expression.ds",
            r#"
`value`;
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_function_call() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_allows_function_call.ds",
            r#"
doSomething();
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_allows_assignment.ds",
            r#"
x = 5;
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_void_unary_statement() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_allows_void_unary_statement.ds",
            r#"
void maybeValue;
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_module_directive_prologue_strings() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_allows_module_directive_prologue_strings.ds",
            r#"
"use strict";
"use custom";
doSomething();
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_reports_string_after_non_directive_statement() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_reports_string_after_non_directive_statement.ds",
            r#"
doSomething();
"use strict";
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }
}
