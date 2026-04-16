use crate::LintMeta;
use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    walk_expression, walk_member, walk_property,
};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, expression_starts_nested_declaration_scope,
    expression_unwrap_statement_source_form, for_each_callable_signature,
};
use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of return statements per function.
    ///
    /// Functions with many return points can be harder to follow and maintain.
    /// Consider extracting logic or reducing branching depth.
    #[lint(
        id = "max-return-statements",
        code = "LX009",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxReturnStatements,
    "Limit return statements per function"
}

impl LintRule for MaxReturnStatements {
    fn meta(&self) -> &'static LintMeta {
        MaxReturnStatements::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_return_statements = ctx.options.complexity.max_return_statements;

        // check callable bodies across declarations and methods
        for_each_callable_signature(ctx.tree, |owner_id, _signature, body_id| {
            // skip declaration only callables
            let Some(body_id) = body_id else {
                return;
            };

            // count explicit returns in this callable body
            let return_count = count_callable_returns(ctx.tree, body_id);
            if return_count <= max_return_statements {
                return;
            }

            // report declaration owner violations
            if let CallableOwnerId::Declaration(declaration_id) = owner_id {
                report_return_limit_violation(
                    ctx,
                    meta,
                    declaration_id,
                    body_id,
                    return_count,
                    max_return_statements,
                );
                return;
            }

            // report member owner violations
            if let CallableOwnerId::Member(member_id) = owner_id {
                report_return_limit_violation(
                    ctx,
                    meta,
                    member_id,
                    body_id,
                    return_count,
                    max_return_statements,
                );
                return;
            }

            // report property owner violations
            if let CallableOwnerId::Property(property_id) = owner_id {
                report_return_limit_violation(
                    ctx,
                    meta,
                    property_id,
                    body_id,
                    return_count,
                    max_return_statements,
                );
            }
        });
    }
}

/// Count explicit return expressions for one callable body.
fn count_callable_returns(tree: &NodeTree, body_expression_id: LocalNodeId<Expression>) -> usize {
    // initialize return counter visitor
    let mut visitor = ReturnCountVisitor {
        options: NodeVisitorOptions::default(),
        root_expression_id: body_expression_id,
        return_count: 0,
    };

    // walk callable body subtree
    let body_expression = tree.get(body_expression_id);
    visitor.visit_expression(tree, body_expression_id, body_expression);

    visitor.return_count
}

/// Report one max-return-statements violation.
fn report_return_limit_violation<T: ast::Node>(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
    owner_id: ast::LocalNodeId<T>,
    body_id: ast::LocalNodeId<ast::Expression>,
    return_count: usize,
    max_return_statements: usize,
) {
    // resolve effective severity
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // report one return overflow diagnostic
    ctx.report(
        LintDiagnostic::new(
            MAX_RETURN_STATEMENTS.id,
            MAX_RETURN_STATEMENTS.code,
            MAX_RETURN_STATEMENTS.category,
            severity,
            format!("function has {return_count} return statements (max {max_return_statements})"),
            ctx.module.file_id,
            ctx.tree.get_span(body_id),
        )
        .with_label("consider reducing return points"),
    );
}

/// Visitor that counts return statements in one callable body.
struct ReturnCountVisitor {
    /// Traversal options.
    options: NodeVisitorOptions,
    /// Root callable body expression.
    root_expression_id: LocalNodeId<Expression>,
    /// Number of explicit returns.
    return_count: usize,
}

impl NodeVisitor for ReturnCountVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // keep nested declaration scopes out of this callable count
        if expression_id != self.root_expression_id {
            let normalized_expression_id =
                expression_unwrap_statement_source_form(tree, expression_id);
            let normalized_expression = tree.get(normalized_expression_id);
            if expression_starts_nested_declaration_scope(normalized_expression) {
                return;
            }
        }

        // count explicit return statements
        if matches!(expression, Expression::Return { .. }) {
            self.return_count += 1;
        }

        // recurse through expression subtree
        walk_expression(self, tree, expression_id, expression);
    }

    fn visit_property(
        &mut self,
        tree: &NodeTree,
        property_id: LocalNodeId<ast::Property>,
        property: &ast::Property,
    ) {
        // keep nested object methods out of parent callable counts
        if matches!(property, ast::Property::Method { .. }) {
            return;
        }

        // recurse into non method properties
        walk_property(self, tree, property_id, property);
    }

    fn visit_member(
        &mut self,
        tree: &NodeTree,
        member_id: LocalNodeId<ast::Member>,
        member: &ast::Member,
    ) {
        // keep nested callable members out of parent callable counts
        if matches!(
            member,
            ast::Member::Method { .. }
                | ast::Member::StaticBlock { .. }
                | ast::Member::ComptimeBlock { .. }
        ) {
            return;
        }

        // recurse into non callable members
        walk_member(self, tree, member_id, member);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_returns() {
        let test = TestProgram::for_rule_without_prelude(MaxReturnStatements);
        let result = test.lint_ast(
            "max_return_statements/test_detects_too_many_returns.ds",
            r#"
function tooManyReturns(x: int32): int32 {
    if (x < 0) { return -1; }
    if (x == 0) { return 0; }
    if (x == 1) { return 1; }
    if (x == 2) { return 2; }
    if (x == 3) { return 3; }
    if (x == 4) { return 4; }
    if (x == 5) { return 5; }
    if (x == 6) { return 6; }
    if (x == 7) { return 7; }
    if (x == 8) { return 8; }
    return 10;
}
"#,
        );
        test.result(result).assert_lint("max-return-statements");
    }

    #[test]
    fn test_allows_few_returns() {
        let test = TestProgram::for_rule_without_prelude(MaxReturnStatements);
        let result = test.lint_ast(
            "max_return_statements/test_allows_few_returns.ds",
            r#"
function fewReturns(x: int32): int32 {
    if (x < 0) { return -1; }
    if (x == 0) { return 0; }
    return x;
}
"#,
        );
        test.result(result).assert_no_lint("max-return-statements");
    }

    #[test]
    fn test_counts_returns_in_match() {
        let test = TestProgram::for_rule_without_prelude(MaxReturnStatements);
        let result = test.lint_ast(
            "max_return_statements/test_counts_returns_in_match.ds",
            r#"
function matchReturns(x: int32): int32 {
    match (x) {
        0 => return 0
        1 => return 1
        2 => return 2
        3 => return 3
        4 => return 4
        5 => return 5
        6 => return 6
        7 => return 7
        8 => return 8
        9 => return 9
        _ => return 10
    }
}
"#,
        );
        test.result(result).assert_lint("max-return-statements");
    }

    #[test]
    fn test_ignores_nested_function_returns_for_outer_function() {
        let test = TestProgram::for_rule_without_prelude(MaxReturnStatements)
            .with_options(|options| options.complexity.max_return_statements = 1);
        let result = test.lint_ast(
            "max_return_statements/test_ignores_nested_function_returns_for_outer_function.ds",
            r#"
function outer(x: int32): int32 {
    function inner(y: int32): int32 {
        if (y == 0) { return 0; }
        if (y == 1) { return 1; }
        return 2;
    }
    return inner(x);
}
"#,
        );
        test.result(result)
            .assert_lint("max-return-statements")
            .assert_lint_count("max-return-statements", 1);
    }

    #[test]
    fn test_detects_object_method_too_many_returns() {
        let test = TestProgram::for_rule_without_prelude(MaxReturnStatements)
            .with_options(|options| options.complexity.max_return_statements = 2);
        let result = test.lint_ast(
            "max_return_statements/test_detects_object_method_too_many_returns.ds",
            r#"
const service = {
    run(x: int32): int32 {
        if (x == 0) { return 0; }
        if (x == 1) { return 1; }
        return 2;
    }
}
"#,
        );
        test.result(result).assert_lint("max-return-statements");
    }
}
