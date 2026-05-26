use crate::LintMeta;
use destack_dir::{self as dir, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_has_side_effects, expression_unwrap_parenthesized_source_form,
};
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow statement expressions that have no effect.
    ///
    /// Expressions that are evaluated and discarded are often mistakes or leftovers from refactors.
    #[lint(
        id = "no-unused-expressions",
        code = "LX022",
        category = Complexity,
        level = Dir,
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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
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
    Block(dir::LocalNodeId<dir::Block>),
    /// Global declaration expressions.
    Declaration(dir::LocalNodeId<dir::Declaration>),
}

/// Collect every statement-expression candidate in one module.
fn collect_statement_expression_ids(
    ctx: &LintModuleContext<'_>,
) -> Vec<dir::LocalNodeId<dir::Expression>> {
    let mut statement_expression_ids = Vec::new();

    // module roots
    for statement_expression_id in ctx.roots.iter().copied() {
        statement_expression_ids.push(statement_expression_id);
    }

    // block statement lists
    for block_id in ctx.dir.iter_nodes::<dir::Block>() {
        let block = ctx.dir.get(block_id);
        for statement_expression_id in block.iter_expressions() {
            statement_expression_ids.push(statement_expression_id);
        }
    }

    // global statement lists
    for declaration_id in ctx.dir.iter_nodes::<dir::Declaration>() {
        let declaration = ctx.dir.get(declaration_id);
        match declaration {
            dir::Declaration::Global(declaration) => {
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
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    statement_expression_id: dir::LocalNodeId<dir::Expression>,
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
            ctx.dir.get_span(statement_expression_id),
        )
        .label("expected an assignment or call in statement position"),
    );
}

/// Return one inner expression id when an expression is statement like.
fn statement_expression_inner_id(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // keep only implicit expression statements at root and block level
    if ctx.dir.get(expression_id).is_top_level_statement() {
        return None;
    }

    Some(expression_id)
}

/// Return true when one statement expression is in a directive prologue.
fn expression_statement_is_directive(
    ctx: &LintModuleContext<'_>,
    statement_expression_id: dir::LocalNodeId<dir::Expression>,
    inner_expression_id: dir::LocalNodeId<dir::Expression>,
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
    ctx: &LintModuleContext<'_>,
    statement_prefix: &[dir::LocalNodeId<dir::Expression>],
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
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    matches!(
        ctx.dir.get(expression_id),
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(_))
    )
}

/// Return the surrounding statement list owner and one statement index.
fn statement_list_owner_and_index(
    ctx: &LintModuleContext<'_>,
    statement_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(StatementListOwner, usize)> {
    // module roots
    if let Some(statement_index) = expression_index_in_slice(&ctx.roots, statement_expression_id) {
        return Some((StatementListOwner::Module, statement_index));
    }

    // block bodies
    if let Some((block_id, statement_index)) =
        block_statement_list_index(ctx, statement_expression_id)
    {
        return Some((StatementListOwner::Block(block_id), statement_index));
    }

    // global bodies
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
    ctx: &LintModuleContext<'_>,
    owner: StatementListOwner,
) -> Vec<dir::LocalNodeId<dir::Expression>> {
    match owner {
        StatementListOwner::Module => ctx.roots.to_vec(),
        StatementListOwner::Block(block_id) => ctx
            .dir
            .tree()
            .get(block_id)
            .iter_expressions()
            .collect::<Vec<_>>(),
        StatementListOwner::Declaration(declaration_id) => {
            let declaration = ctx.dir.get(declaration_id);
            match declaration {
                dir::Declaration::Global(declaration) => declaration.expressions.clone(),
                _ => Vec::new(),
            }
        }
    }
}

/// Return true when one statement list owner can host directives.
fn statement_list_owner_is_directive_capable(
    ctx: &LintModuleContext<'_>,
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
    ctx: &LintModuleContext<'_>,
    statement_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(dir::LocalNodeId<dir::Block>, usize)> {
    // require a block parent for statement expressions
    let block_id = ctx.dir.get_parent_id(statement_expression_id.id)?;
    if ctx.dir.get_node_type(block_id) != dir::NodeType::Block {
        return None;
    }
    let block_id = dir::LocalNodeId::<dir::Block>::new(block_id);
    let block = ctx.dir.get(block_id);
    let expression_ids = block.iter_expressions().collect::<Vec<_>>();
    let statement_index = expression_index_in_slice(&expression_ids, statement_expression_id)?;

    Some((block_id, statement_index))
}

/// Return declaration id and statement index when one statement belongs to one declaration body.
fn declaration_statement_list_index(
    ctx: &LintModuleContext<'_>,
    statement_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(dir::LocalNodeId<dir::Declaration>, usize)> {
    // require a declaration parent for global bodies
    let declaration_id = ctx.dir.get_parent_id(statement_expression_id.id)?;
    if ctx.dir.get_node_type(declaration_id) != dir::NodeType::Declaration {
        return None;
    }

    let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(declaration_id);
    let declaration = ctx.dir.get(declaration_id);
    let expressions = match declaration {
        dir::Declaration::Global(declaration) => declaration.expressions.as_slice(),
        _ => return None,
    };
    let statement_index = expression_index_in_slice(expressions, statement_expression_id)?;

    Some((declaration_id, statement_index))
}

/// Return true when one block can host a directive prologue.
fn block_statement_list_is_directive_capable(
    ctx: &LintModuleContext<'_>,
    block_id: dir::LocalNodeId<dir::Block>,
) -> bool {
    // resolve the parent expression for this block
    let Some(block_expression_raw_id) = ctx.dir.get_parent_id(block_id.id) else {
        return false;
    };
    if ctx.dir.get_node_type(block_expression_raw_id) != dir::NodeType::Expression {
        return false;
    }
    let block_expression_id = dir::LocalNodeId::<dir::Expression>::new(block_expression_raw_id);
    let block_expression = ctx.dir.get(block_expression_id);
    if !matches!(block_expression, dir::Expression::Block(inner) if *inner == block_id) {
        return false;
    }

    // resolve owner node for this block expression
    let Some(owner_raw_id) = ctx.dir.get_parent_id(block_expression_id.id) else {
        return false;
    };
    let owner_type = ctx.dir.get_node_type(owner_raw_id);

    // allow function declaration bodies
    if owner_type == dir::NodeType::Declaration {
        let owner_id = dir::LocalNodeId::<dir::Declaration>::new(owner_raw_id);
        let owner = ctx.dir.get(owner_id);
        return matches!(
            owner,
            dir::Declaration::Function(dir::FunctionDeclaration {
                body: Some(body_expression_id),
                ..
            }) if *body_expression_id == block_expression_id
        );
    }

    // allow class and interface method bodies
    if owner_type == dir::NodeType::Member {
        let owner_id = dir::LocalNodeId::<dir::Member>::new(owner_raw_id);
        let owner = ctx.dir.get(owner_id);
        return matches!(
            owner,
            dir::Member::Method {
                body: Some(body_expression_id),
                ..
            } if *body_expression_id == block_expression_id
        );
    }

    // allow object method bodies
    if owner_type == dir::NodeType::Property {
        let owner_id = dir::LocalNodeId::<dir::Property>::new(owner_raw_id);
        let owner = ctx.dir.get(owner_id);
        return matches!(
            owner,
            dir::Property::Method {
                body: Some(body_expression_id),
                ..
            } if *body_expression_id == block_expression_id
        );
    }

    false
}

/// Return true when one expression shape is disallowed as a statement expression.
fn expression_is_disallowed_in_statement(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // normalize transparent wrappers first
    let expression_id = statement_expression_subject_id(ctx, expression_id);
    let expression = ctx.dir.get(expression_id);

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
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    let mut current_expression_id =
        expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);

    loop {
        let current_expression = ctx.dir.get(current_expression_id);
        match current_expression {
            dir::Expression::Must { left, .. } | dir::Expression::Maybe { left, .. } => {
                current_expression_id =
                    expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *left);
            }
            dir::Expression::Instantiation { left, .. } => {
                current_expression_id =
                    expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *left);
            }
            dir::Expression::BorrowOf { right, .. } | dir::Expression::MoveOf { right, .. } => {
                current_expression_id =
                    expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *right);
            }
            _ => return current_expression_id,
        }
    }
}

/// Return one option-sensitive classification result when the rule has a special case.
fn expression_disallowed_by_rule_options(
    ctx: &LintModuleContext<'_>,
    expression: &dir::Expression,
) -> Option<bool> {
    match expression {
        dir::Expression::TaggedTemplateExpression { .. } => Some(
            !ctx.options
                .complexity
                .no_unused_expressions_allow_tagged_templates,
        ),
        dir::Expression::TreeExpression { .. } => {
            Some(ctx.options.complexity.no_unused_expressions_enforce_for_jsx)
        }
        dir::Expression::If {
            form: dir::IfForm::Ternary,
            then_expression,
            else_expression: Some(else_expression),
            ..
        } if ctx.options.complexity.no_unused_expressions_allow_ternary => Some(
            expression_is_disallowed_in_statement(ctx, *then_expression)
                || expression_is_disallowed_in_statement(ctx, *else_expression),
        ),
        dir::Expression::Binary {
            operator, right, ..
        } if ctx
            .options
            .complexity
            .no_unused_expressions_allow_short_circuit
            && matches!(operator, dir::BinaryOperator::And | dir::BinaryOperator::Or) =>
        {
            Some(expression_is_disallowed_in_statement(ctx, *right))
        }
        dir::Expression::Unary { operator, .. } => Some(!matches!(
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
fn expression_is_known_effectful_statement(expression: &dir::Expression) -> bool {
    matches!(
        expression,
        dir::Expression::Call { .. }
            | dir::Expression::Assign { .. }
            | dir::Expression::New { .. }
            | dir::Expression::Await { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::AwaitMust { .. }
            | dir::Expression::Yield { .. }
            | dir::Expression::Import { .. }
            | dir::Expression::Export { .. }
            | dir::Expression::Let { .. }
            | dir::Expression::Using { .. }
            | dir::Expression::Declaration(_)
            | dir::Expression::Return { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }
            | dir::Expression::Throw { .. }
            | dir::Expression::If { .. }
            | dir::Expression::For { .. }
            | dir::Expression::ForEach { .. }
            | dir::Expression::While { .. }
            | dir::Expression::Loop { .. }
            | dir::Expression::Match { .. }
            | dir::Expression::Try { .. }
            | dir::Expression::Block(_)
            | dir::Expression::Label { .. }
            | dir::Expression::Debugger
            | dir::Expression::Error
            | dir::Expression::Stub
    )
}

/// Return true when one expression family is obviously pure in statement position.
fn expression_is_known_pure_statement(expression: &dir::Expression) -> bool {
    matches!(
        expression,
        dir::Expression::Identifier { .. }
            | dir::Expression::QualifiedReference { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::PrivateMember { .. }
            | dir::Expression::Index { .. }
            | dir::Expression::Binary { .. }
            | dir::Expression::Type { .. }
            | dir::Expression::ScalarLiteral(_)
            | dir::Expression::ArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. }
            | dir::Expression::StructExpression { .. }
            | dir::Expression::SequenceExpression { .. }
            | dir::Expression::TemplateExpression { .. }
            | dir::Expression::PrivateIdentifier { .. }
            | dir::Expression::ImportMeta
            | dir::Expression::This
            | dir::Expression::Super
    )
}

/// Return index of one expression id in one expression slice.
fn expression_index_in_slice(
    expressions: &[dir::LocalNodeId<dir::Expression>],
    expression_id: dir::LocalNodeId<dir::Expression>,
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "no_unused_expressions/test_reports_tree_expression_when_enforced.ds",
            r#"
<View />;
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_reports_module_directive_when_disabled() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExpressions);
        let result = test.lint(
            "no_unused_expressions/test_reports_module_directive_when_disabled.ts",
            r#"
"use strict";
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }
}
