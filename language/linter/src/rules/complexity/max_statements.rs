use crate::LintMeta;
use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeVisitor, NodeVisitorOptions, Tree, walk_expression,
    walk_member, walk_property,
};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, callable_owner_span, expression_starts_nested_declaration_scope,
    expression_unwrap_statement_source_form, for_each_callable_signature,
};
use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of statements per function.
    ///
    /// Functions with many statements are harder to understand and maintain.
    /// Consider extracting logic into helper functions.
    #[lint(
        id = "max-statements",
        code = "LX010",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxStatements,
    "Limit statements per function"
}

impl LintRule for MaxStatements {
    fn meta(&self) -> &'static LintMeta {
        MaxStatements::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_statements = ctx.options.complexity.max_statements;
        let mut pending_top_level_violations = Vec::new();

        // check all callable owners that have a body expression
        for_each_callable_signature(ctx.tree, |owner_id, _signature, body_id| {
            // skip declaration only callables
            let Some(body_id) = body_id else {
                return;
            };

            // count statements in this callable body, excluding nested callables
            let statement_count = count_callable_statements(ctx.tree, body_id);
            if statement_count <= max_statements {
                return;
            }

            // defer a single top-level function violation
            if ctx
                .options
                .complexity
                .max_statements_ignore_top_level_functions
                && callable_owner_is_top_level_function(ctx.tree, owner_id)
            {
                pending_top_level_violations.push((owner_id, body_id, statement_count));
                return;
            }

            // report declaration owner violations
            if let CallableOwnerId::Declaration(declaration_id) = owner_id {
                report_statement_limit_violation(
                    ctx,
                    meta,
                    declaration_id,
                    body_id,
                    statement_count,
                    max_statements,
                );
                return;
            }

            // report member owner violations
            if let CallableOwnerId::Member(member_id) = owner_id {
                report_statement_limit_violation(
                    ctx,
                    meta,
                    member_id,
                    body_id,
                    statement_count,
                    max_statements,
                );
                return;
            }

            // report property owner violations
            if let CallableOwnerId::Property(property_id) = owner_id {
                report_statement_limit_violation(
                    ctx,
                    meta,
                    property_id,
                    body_id,
                    statement_count,
                    max_statements,
                );
            }
        });

        // ignore exactly one top-level function, not every top-level function
        if pending_top_level_violations.len() == 1 {
            return;
        }

        for (owner_id, body_id, statement_count) in pending_top_level_violations {
            match owner_id {
                CallableOwnerId::Declaration(declaration_id) => {
                    report_statement_limit_violation(
                        ctx,
                        meta,
                        declaration_id,
                        body_id,
                        statement_count,
                        max_statements,
                    );
                }
                CallableOwnerId::Member(member_id) => {
                    report_statement_limit_violation(
                        ctx,
                        meta,
                        member_id,
                        body_id,
                        statement_count,
                        max_statements,
                    );
                }
                CallableOwnerId::Property(property_id) => {
                    report_statement_limit_violation(
                        ctx,
                        meta,
                        property_id,
                        body_id,
                        statement_count,
                        max_statements,
                    );
                }
            }
        }
    }
}

/// Return true when one callable owner is a top-level function.
fn callable_owner_is_top_level_function(tree: &Tree, owner_id: CallableOwnerId) -> bool {
    let owner_span = callable_owner_span(tree, owner_id);
    !callable_is_nested_in_enclosing_scope(tree, owner_span)
}

/// Return true when one callable span is nested inside another callable-like scope.
fn callable_is_nested_in_enclosing_scope(tree: &Tree, owner_span: Span) -> bool {
    // nested function declarations
    for enclosing_declaration_id in tree.iter_nodes::<ast::Declaration>() {
        let enclosing_declaration = tree.get(enclosing_declaration_id);
        if !matches!(enclosing_declaration, ast::Declaration::Function(_)) {
            continue;
        }

        let enclosing_span = tree.get_span(enclosing_declaration_id);
        if span_strictly_contains(enclosing_span, owner_span) {
            return true;
        }
    }

    // class methods and static or comptime blocks
    for member_id in tree.iter_nodes::<ast::Member>() {
        let member = tree.get(member_id);
        if !matches!(
            member,
            ast::Member::Method { .. }
                | ast::Member::StaticBlock { .. }
                | ast::Member::ComptimeBlock { .. }
        ) {
            continue;
        }

        let enclosing_span = tree.get_span(member_id);
        if span_strictly_contains(enclosing_span, owner_span) {
            return true;
        }
    }

    // object methods
    for property_id in tree.iter_nodes::<ast::Property>() {
        let property = tree.get(property_id);
        if !matches!(property, ast::Property::Method { .. }) {
            continue;
        }

        let enclosing_span = tree.get_span(property_id);
        if span_strictly_contains(enclosing_span, owner_span) {
            return true;
        }
    }

    false
}

/// Return true when one span strictly contains another span.
fn span_strictly_contains(outer: Span, inner: Span) -> bool {
    outer.file == inner.file
        && outer.start <= inner.start
        && inner.end <= outer.end
        && outer != inner
}

/// Count statements for one callable body while skipping nested callable scopes.
fn count_callable_statements(tree: &Tree, body_expression_id: LocalNodeId<Expression>) -> usize {
    // initialize statement count visitor for one callable body
    let mut visitor = StatementCountVisitor {
        options: NodeVisitorOptions::default(),
        root_expression_id: body_expression_id,
        statement_count: 0,
    };

    // walk the callable body subtree
    let body_expression = tree.get(body_expression_id);
    visitor.visit_expression(tree, body_expression_id, body_expression);

    visitor.statement_count
}

/// Report one max-statements violation for a callable owner.
fn report_statement_limit_violation<T: ast::Node>(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
    owner_id: ast::LocalNodeId<T>,
    body_id: ast::LocalNodeId<ast::Expression>,
    statement_count: usize,
    max_statements: usize,
) {
    // resolve effective severity for this owner node
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // report one statement count overflow diagnostic
    ctx.report(
        LintDiagnostic::new(
            MAX_STATEMENTS.id,
            MAX_STATEMENTS.code,
            MAX_STATEMENTS.category,
            severity,
            format!("function has {statement_count} statements (max {max_statements})"),
            ctx.module.file_id,
            ctx.tree.get_span(body_id),
        )
        .with_label("consider breaking into smaller functions"),
    );
}

/// Visitor that counts statements inside one callable body.
struct StatementCountVisitor {
    /// Traversal options.
    options: NodeVisitorOptions,
    /// Root callable body expression.
    root_expression_id: LocalNodeId<Expression>,
    /// Number of statements counted in the callable body.
    statement_count: usize,
}

impl NodeVisitor for StatementCountVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
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

        // count statements from each block expression
        if let Expression::Block(block_id) = expression {
            let block = tree.get(*block_id);
            self.statement_count += block.len();
        }

        // recurse into expression children
        walk_expression(self, tree, expression_id, expression);
    }

    fn visit_property(
        &mut self,
        tree: &Tree,
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
        tree: &Tree,
        member_id: LocalNodeId<ast::Member>,
        member: &ast::Member,
    ) {
        // keep nested member callable scopes out of parent callable counts
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
    fn test_detects_too_many_statements() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements);

        // create a function with 51 statements, over default 50
        let mut source = String::from("function foo() {\n");
        for i in 0..51 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("}\n");

        // verify lint report for oversized body
        let result = test.lint_ast(
            "max_statements/test_detects_too_many_statements.ds",
            &source,
        );
        test.result(result).assert_lint("max-statements");
    }

    #[test]
    fn test_allows_few_statements() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements);

        // keep the statement count within default threshold
        let result = test.lint_ast(
            "max_statements/test_allows_few_statements.ds",
            r#"
function foo() {
    let x = 1;
    let y = 2;
    return x + y;
}
"#,
        );

        // verify no lint for small body
        test.result(result).assert_no_lint("max-statements");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements);

        // create a function with exactly 50 statements
        let mut source = String::from("function foo() {\n");
        for i in 0..50 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("}\n");

        // verify no lint at exact threshold
        let result = test.lint_ast("max_statements/test_allows_exactly_at_limit.ds", &source);
        test.result(result).assert_no_lint("max-statements");
    }

    #[test]
    fn test_counts_nested_block_statements() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements)
            .with_options(|options| options.complexity.max_statements = 2);

        // include nested block statements in the same callable
        let result = test.lint_ast(
            "max_statements/test_counts_nested_block_statements.ds",
            r#"
function foo(flag: boolean) {
    if (flag) {
        let x = 1;
        let y = 2;
    }
}
"#,
        );

        // verify nested blocks contribute to the total
        test.result(result).assert_lint("max-statements");
    }

    #[test]
    fn test_expression_body_lambda_counts_zero_statements() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements)
            .with_options(|options| options.complexity.max_statements = 0);

        // expression body lambdas have no block statements
        let result = test.lint_ast(
            "max_statements/test_expression_body_lambda_counts_zero_statements.ds",
            r#"
const fn = (value: int32) => value + 1;
"#,
        );

        // verify no lint for expression body lambda
        test.result(result).assert_no_lint("max-statements");
    }

    #[test]
    fn test_ignores_nested_function_statements() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements)
            .with_options(|options| options.complexity.max_statements = 1);

        // ignore statements inside nested callable scopes
        let result = test.lint_ast(
            "max_statements/test_ignores_nested_function_statements.ds",
            r#"
function outer() {
    function inner() {
        let x = 1;
    }
}
"#,
        );

        // verify nested function body does not inflate outer count
        test.result(result).assert_no_lint("max-statements");
    }

    #[test]
    fn test_detects_object_method_too_many_statements() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements)
            .with_options(|options| options.complexity.max_statements = 2);

        // count statements for object method bodies
        let result = test.lint_ast(
            "max_statements/test_detects_object_method_too_many_statements.ds",
            r#"
const object = {
    method() {
        let a = 1;
        let b = 2;
        let c = 3;
    }
}
"#,
        );

        // verify lint report for oversized object method
        test.result(result).assert_lint("max-statements");
    }

    #[test]
    fn test_ignores_top_level_function_declarations_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements).with_options(|options| {
            options.complexity.max_statements = 1;
            options.complexity.max_statements_ignore_top_level_functions = true;
        });

        let result = test.lint_ast(
            "max_statements/test_ignores_top_level_function_declarations_when_enabled.ds",
            r#"
function outer() {
    let x = 1;
    let y = 2;
}
"#,
        );

        test.result(result).assert_no_lint("max-statements");
    }

    #[test]
    fn test_ignores_single_top_level_wrapper_function_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements).with_options(|options| {
            options.complexity.max_statements = 1;
            options.complexity.max_statements_ignore_top_level_functions = true;
        });

        let result = test.lint_ast(
            "max_statements/test_ignores_single_top_level_wrapper_function_when_enabled.ds",
            r#"
register(() => {
    let x = 1;
    let y = 2;
});
"#,
        );

        test.result(result).assert_no_lint("max-statements");
    }

    #[test]
    fn test_reports_multiple_top_level_functions_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements).with_options(|options| {
            options.complexity.max_statements = 1;
            options.complexity.max_statements_ignore_top_level_functions = true;
        });

        let result = test.lint_ast(
            "max_statements/test_reports_multiple_top_level_functions_when_enabled.ds",
            r#"
first(() => {
    let x = 1;
    let y = 2;
});

second(() => {
    let a = 1;
    let b = 2;
});
"#,
        );

        test.result(result)
            .assert_lint("max-statements")
            .assert_lint_count("max-statements", 2);
    }

    #[test]
    fn test_ignores_single_top_level_object_method_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(MaxStatements).with_options(|options| {
            options.complexity.max_statements = 1;
            options.complexity.max_statements_ignore_top_level_functions = true;
        });

        let result = test.lint_ast(
            "max_statements/test_ignores_single_top_level_object_method_when_enabled.ds",
            r#"
const object = {
    method() {
        let x = 1;
        let y = 2;
    }
};
"#,
        );

        test.result(result).assert_no_lint("max-statements");
    }
}
