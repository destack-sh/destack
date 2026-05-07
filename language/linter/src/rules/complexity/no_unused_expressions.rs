use crate::LintMeta;
use destack_ast::{self as ast, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_has_side_effects, expression_unwrap_parenthesized_source_form,
};
use crate::{LintAstContext, LintReport, LintRule, declare_lint};

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
    fn meta(&self) -> &'static LintMeta {
        NoUnusedExpressions::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // inspect every statement-expression candidate in the module
        for statement_expression_id in collect_statement_expression_ids(ctx) {
            check_statement_candidate(ctx, meta, statement_expression_id);
        }
    }
}

/// One owner of a statement-expression list.
#[derive(Debug, Copy, Clone)]
enum StatementListOwner {
    /// Module root expressions.
    Module,
    /// Block body expressions.
    Block(ast::LocalNodeId<ast::Block>),
    /// Namespace or global declaration expressions.
    Declaration(ast::LocalNodeId<ast::Declaration>),
}

/// Collect every statement-expression candidate in one module.
fn collect_statement_expression_ids(
    ctx: &LintAstContext<'_>,
) -> Vec<ast::LocalNodeId<ast::Expression>> {
    let mut statement_expression_ids = Vec::new();

    // module roots
    for statement_expression_id in ctx.roots {
        statement_expression_ids.push(*statement_expression_id);
    }

    // block statement lists
    for block_id in ctx.tree.iter_nodes::<ast::Block>() {
        let block = ctx.tree.get(block_id);
        for statement_expression_id in block.iter_expressions() {
            statement_expression_ids.push(statement_expression_id);
        }
    }

    // namespace and global statement lists
    for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
        let declaration = ctx.tree.get(declaration_id);
        match declaration {
            ast::Declaration::Global(declaration) => {
                for statement_expression_id in &declaration.expressions {
                    statement_expression_ids.push(*statement_expression_id);
                }
            }
            ast::Declaration::Module(declaration) => {
                for statement_expression_id in &declaration.expressions {
                    statement_expression_ids.push(*statement_expression_id);
                }
            }
            ast::Declaration::Namespace(declaration) => {
                for statement_expression_id in &declaration.expressions {
                    statement_expression_ids.push(*statement_expression_id);
                }
            }
            _ => {}
        }
    }

    statement_expression_ids
}

/// Check one root or block expression as a statement candidate.
fn check_statement_candidate(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
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
        LintReport::new(
            NO_UNUSED_EXPRESSIONS.id,
            NO_UNUSED_EXPRESSIONS.code,
            NO_UNUSED_EXPRESSIONS.category,
            severity,
            "expression statement has no effect",
            ctx.tree.get_span(statement_expression_id),
        )
        .label("expected an assignment or call in statement position"),
    );
}

/// Return one inner expression id when an expression is statement like.
fn statement_expression_inner_id(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    // keep only implicit expression statements at root and block level
    if ctx.tree.get(expression_id).is_top_level_statement() {
        return None;
    }

    Some(expression_id)
}

/// Return true when one statement expression is in a directive prologue.
fn expression_statement_is_directive(
    ctx: &LintAstContext<'_>,
    statement_expression_id: ast::LocalNodeId<ast::Expression>,
    inner_expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // allow callers to disable directive handling
    if !ctx
        .options
        .complexity
        .no_unused_expressions_ignore_directives
    {
        return false;
    }

    // directives are string literals only
    if !expression_is_string_literal_statement(ctx, inner_expression_id) {
        return false;
    }

    // resolve the surrounding statement list and its prefix
    let Some((owner, statement_index)) =
        statement_list_owner_and_index(ctx, statement_expression_id)
    else {
        return false;
    };
    if !statement_list_owner_is_directive_capable(ctx, owner) {
        return false;
    }

    let expressions = statement_list_owner_expressions(ctx, owner);
    expression_prefix_is_all_directives(ctx, &expressions[..statement_index])
}

/// Return true when all statement expressions in one prefix are directive literals.
fn expression_prefix_is_all_directives(
    ctx: &LintAstContext<'_>,
    statement_prefix: &[ast::LocalNodeId<ast::Expression>],
) -> bool {
    // require every earlier statement to be a string literal statement
    statement_prefix.iter().all(|expression_id| {
        let Some(inner_expression_id) = statement_expression_inner_id(ctx, *expression_id) else {
            return false;
        };

        expression_is_string_literal_statement(ctx, inner_expression_id)
    })
}

/// Return true when one expression is a string literal statement payload.
fn expression_is_string_literal_statement(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.tree, expression_id);
    matches!(
        ctx.tree.get(expression_id),
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_))
    )
}

/// Return the surrounding statement list owner and one statement index.
fn statement_list_owner_and_index(
    ctx: &LintAstContext<'_>,
    statement_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<(StatementListOwner, usize)> {
    // module roots
    if let Some(statement_index) = expression_index_in_slice(ctx.roots, statement_expression_id) {
        return Some((StatementListOwner::Module, statement_index));
    }

    // block bodies
    if let Some((block_id, statement_index)) =
        block_statement_list_index(ctx, statement_expression_id)
    {
        return Some((StatementListOwner::Block(block_id), statement_index));
    }

    // namespace and global bodies
    if let Some((declaration_id, statement_index)) =
        declaration_statement_list_index(ctx, statement_expression_id)
    {
        return Some((
            StatementListOwner::Declaration(declaration_id),
            statement_index,
        ));
    }

    None
}

/// Return the expression slice for one statement list owner.
fn statement_list_owner_expressions(
    ctx: &LintAstContext<'_>,
    owner: StatementListOwner,
) -> Vec<ast::LocalNodeId<ast::Expression>> {
    match owner {
        StatementListOwner::Module => ctx.roots.to_vec(),
        StatementListOwner::Block(block_id) => ctx
            .tree
            .get(block_id)
            .iter_expressions()
            .collect::<Vec<_>>(),
        StatementListOwner::Declaration(declaration_id) => {
            let declaration = ctx.tree.get(declaration_id);
            match declaration {
                ast::Declaration::Global(declaration) => declaration.expressions.clone(),
                ast::Declaration::Module(declaration) => declaration.expressions.clone(),
                ast::Declaration::Namespace(declaration) => declaration.expressions.clone(),
                _ => Vec::new(),
            }
        }
    }
}

/// Return true when one statement list owner can host directives.
fn statement_list_owner_is_directive_capable(
    ctx: &LintAstContext<'_>,
    owner: StatementListOwner,
) -> bool {
    match owner {
        StatementListOwner::Module => true,
        StatementListOwner::Block(block_id) => {
            block_statement_list_is_directive_capable(ctx, block_id)
        }
        StatementListOwner::Declaration(_) => true,
    }
}

/// Return block id and statement index when one statement belongs to one block body.
fn block_statement_list_index(
    ctx: &LintAstContext<'_>,
    statement_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<(ast::LocalNodeId<ast::Block>, usize)> {
    // require a block parent for statement expressions
    let block_id = ctx.parents.get(statement_expression_id)?;
    if ctx.tree.get_node_type(block_id) != ast::NodeType::Block {
        return None;
    }
    let block_id = ast::LocalNodeId::<ast::Block>::new(block_id);
    let block = ctx.tree.get(block_id);
    let expression_ids = block.iter_expressions().collect::<Vec<_>>();
    let statement_index = expression_index_in_slice(&expression_ids, statement_expression_id)?;

    Some((block_id, statement_index))
}

/// Return declaration id and statement index when one statement belongs to one declaration body.
fn declaration_statement_list_index(
    ctx: &LintAstContext<'_>,
    statement_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<(ast::LocalNodeId<ast::Declaration>, usize)> {
    // require a declaration parent for namespace and global bodies
    let declaration_id = ctx.parents.get(statement_expression_id)?;
    if ctx.tree.get_node_type(declaration_id) != ast::NodeType::Declaration {
        return None;
    }

    let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(declaration_id);
    let declaration = ctx.tree.get(declaration_id);
    let expressions = match declaration {
        ast::Declaration::Global(declaration) => declaration.expressions.as_slice(),
        ast::Declaration::Module(declaration) => declaration.expressions.as_slice(),
        ast::Declaration::Namespace(declaration) => declaration.expressions.as_slice(),
        _ => return None,
    };
    let statement_index = expression_index_in_slice(expressions, statement_expression_id)?;

    Some((declaration_id, statement_index))
}

/// Return true when one block can host a directive prologue.
fn block_statement_list_is_directive_capable(
    ctx: &LintAstContext<'_>,
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
            ast::Declaration::Function(ast::FunctionDeclaration {
                body: Some(body_expression_id),
                ..
            }) if *body_expression_id == block_expression_id
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
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // normalize transparent wrappers first
    let expression_id = statement_expression_subject_id(ctx, expression_id);
    let expression = ctx.tree.get(expression_id);

    // handle the option-sensitive upstream special cases first
    if let Some(is_disallowed) = expression_disallowed_by_rule_options(ctx, expression) {
        return is_disallowed;
    }

    // handle the obvious effectful statement families
    if expression_is_known_effectful_statement(expression) {
        return false;
    }

    // handle the obvious pure statement families
    if expression_is_known_pure_statement(expression) {
        return true;
    }

    // fall back to semantic side-effect analysis for the rest
    !expression_has_side_effects(ctx, expression_id)
}

/// Return the effective subject expression for statement classification.
fn statement_expression_subject_id(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> ast::LocalNodeId<ast::Expression> {
    let mut current_expression_id =
        expression_unwrap_parenthesized_source_form(ctx.tree, expression_id);

    loop {
        let current_expression = ctx.tree.get(current_expression_id);
        match current_expression {
            ast::Expression::Must { left, .. } | ast::Expression::Maybe { left, .. } => {
                current_expression_id =
                    expression_unwrap_parenthesized_source_form(ctx.tree, *left);
            }
            ast::Expression::Instantiation { left, .. } => {
                current_expression_id =
                    expression_unwrap_parenthesized_source_form(ctx.tree, *left);
            }
            ast::Expression::ReferenceOf { right, .. }
            | ast::Expression::ValueOf { right, .. }
            | ast::Expression::PointerOf { right, .. } => {
                current_expression_id =
                    expression_unwrap_parenthesized_source_form(ctx.tree, *right);
            }
            _ => return current_expression_id,
        }
    }
}

/// Return one option-sensitive classification result when the rule has a special case.
fn expression_disallowed_by_rule_options(
    ctx: &LintAstContext<'_>,
    expression: &ast::Expression,
) -> Option<bool> {
    match expression {
        ast::Expression::Delete { .. } => Some(false),
        ast::Expression::TaggedTemplateExpression { .. } => Some(
            !ctx.options
                .complexity
                .no_unused_expressions_allow_tagged_templates,
        ),
        ast::Expression::TreeExpression { .. } => {
            Some(ctx.options.complexity.no_unused_expressions_enforce_for_jsx)
        }
        ast::Expression::If {
            form: ast::IfForm::Ternary,
            then_expression,
            else_expression: Some(else_expression),
            ..
        } if ctx.options.complexity.no_unused_expressions_allow_ternary => Some(
            expression_is_disallowed_in_statement(ctx, *then_expression)
                || expression_is_disallowed_in_statement(ctx, *else_expression),
        ),
        ast::Expression::Binary {
            operator, right, ..
        } if ctx
            .options
            .complexity
            .no_unused_expressions_allow_short_circuit
            && matches!(operator, ast::BinaryOperator::And | ast::BinaryOperator::Or) =>
        {
            Some(expression_is_disallowed_in_statement(ctx, *right))
        }
        ast::Expression::Unary { operator, .. } => Some(!matches!(
            operator,
            UnaryOperator::PreIncrement
                | UnaryOperator::PostIncrement
                | UnaryOperator::PreDecrement
                | UnaryOperator::PostDecrement
                | UnaryOperator::Void
        )),
        _ => None,
    }
}

/// Return true when one expression family is obviously effectful in statement position.
fn expression_is_known_effectful_statement(expression: &ast::Expression) -> bool {
    matches!(
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
    )
}

/// Return true when one expression family is obviously pure in statement position.
fn expression_is_known_pure_statement(expression: &ast::Expression) -> bool {
    matches!(
        expression,
        ast::Expression::Identifier { .. }
            | ast::Expression::QualifiedReference { .. }
            | ast::Expression::Member { .. }
            | ast::Expression::PrivateMember { .. }
            | ast::Expression::Index { .. }
            | ast::Expression::Binary { .. }
            | ast::Expression::Type { .. }
            | ast::Expression::ScalarLiteral(_)
            | ast::Expression::ArrayExpression { .. }
            | ast::Expression::TupleExpression { .. }
            | ast::Expression::ObjectExpression { .. }
            | ast::Expression::SequenceExpression { .. }
            | ast::Expression::TemplateExpression { .. }
            | ast::Expression::PrivateIdentifier { .. }
            | ast::Expression::ImportMeta
            | ast::Expression::NewTarget
            | ast::Expression::This
            | ast::Expression::Super
    )
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
        let test =
            TestProgram::for_rule_without_prelude(NoUnusedExpressions).with_options(|options| {
                options.complexity.no_unused_expressions_ignore_directives = true
            });
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

    #[test]
    fn test_allows_short_circuit_when_enabled() {
        let test =
            TestProgram::for_rule_without_prelude(NoUnusedExpressions).with_options(|options| {
                options.complexity.no_unused_expressions_allow_short_circuit = true
            });
        let result = test.lint_ast(
            "no_unused_expressions/test_allows_short_circuit_when_enabled.ds",
            r#"
ready && doSomething();
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_ternary_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions)
            .with_options(|options| options.complexity.no_unused_expressions_allow_ternary = true);
        let result = test.lint_ast(
            "no_unused_expressions/test_allows_ternary_when_enabled.ds",
            r#"
ready ? doSomething() : doOtherThing();
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_tagged_templates_when_enabled() {
        let test =
            TestProgram::for_rule_without_prelude(NoUnusedExpressions).with_options(|options| {
                options
                    .complexity
                    .no_unused_expressions_allow_tagged_templates = true
            });
        let result = test.lint_ast(
            "no_unused_expressions/test_allows_tagged_templates_when_enabled.ds",
            r#"
sql`SELECT * FROM users`;
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_tree_expression_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_allows_tree_expression_by_default.ds",
            r#"
<View />;
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_reports_tree_expression_when_enforced() {
        let test =
            TestProgram::for_rule_without_prelude(NoUnusedExpressions).with_options(|options| {
                options.complexity.no_unused_expressions_enforce_for_jsx = true
            });
        let result = test.lint_ast(
            "no_unused_expressions/test_reports_tree_expression_when_enforced.ds",
            r#"
<View />;
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_namespace_directive_prologue_strings() {
        let test =
            TestProgram::for_rule_without_prelude(NoUnusedExpressions).with_options(|options| {
                options.complexity.no_unused_expressions_ignore_directives = true
            });
        let result = test.lint_ast(
            "no_unused_expressions/test_allows_namespace_directive_prologue_strings.ts",
            r#"
namespace Demo {
    "use strict";
    run();
}
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_reports_namespace_string_after_non_directive_statement() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_reports_namespace_string_after_non_directive_statement.ts",
            r#"
namespace Demo {
    run();
    "use strict";
}
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_reports_module_directive_when_disabled() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint_ast(
            "no_unused_expressions/test_reports_module_directive_when_disabled.ts",
            r#"
"use strict";
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }
}
