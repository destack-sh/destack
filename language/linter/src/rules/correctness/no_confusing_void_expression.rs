use destack_dir::{
    self as dir, BinaryOperator, NodeVisitor, NodeVisitorOptions, UnaryOperator, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_is_standalone_statement, expression_outer_transparent_ancestor,
    expression_parent_id, function_return_type, is_void_or_never_type,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `void` expressions where a value is expected.
    ///
    /// `void` is useful as a top-level statement when intentionally discarding
    /// a result.
    /// Nested `void` expressions inside value-producing positions are often
    /// confusing and harder to read.
    #[lint(
        id = "no-confusing-void-expression",
        code = "LC068",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoConfusingVoidExpression,
    "Disallow confusing nested void expressions"
}

impl LintRule for NoConfusingVoidExpression {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoConfusingVoidExpression::meta()
    }

    /// Check module DIR nodes for nested `void` expressions.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // walk expression roots with one visitor pass
        let mut visitor = NoConfusingVoidExpressionVisitor::new(ctx, self.meta());
        visitor.run();
    }
}

/// Node visitor that reports nested void-like expressions in value position.
struct NoConfusingVoidExpressionVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// Whether explicit `void` unary wrappers are ignored.
    ignore_void_operator: bool,
    /// Whether returned void expressions are ignored in void-returning functions.
    ignore_void_returning_functions: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoConfusingVoidExpressionVisitor<'a, 'b> {
    /// Build a visitor for no-confusing-void-expression checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let ignore_void_operator = ctx
            .options
            .correctness
            .no_confusing_void_expression_ignore_void_operator;
        let ignore_void_returning_functions = ctx
            .options
            .correctness
            .no_confusing_void_expression_ignore_void_returning_functions;

        Self {
            ctx,
            meta,
            ignore_void_operator,
            ignore_void_returning_functions,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one expression for confusing value-position void behavior.
    fn check_expression(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // keep only void-like expression candidates
        if !is_void_expression_candidate(self.ctx, expression_id, self.ignore_void_operator) {
            return;
        }

        // keep only value-position usage
        let Some(invalid_ancestor_expression_id) =
            invalid_ancestor_expression_id(self.ctx.tree, expression_id)
        else {
            return;
        };

        // allow returned void expressions in void-returning functions when configured
        if self.ignore_void_returning_functions
            && is_void_returning_function_result_position(self.ctx, invalid_ancestor_expression_id)
        {
            return;
        }

        // honor per-node severity configuration
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report one diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_CONFUSING_VOID_EXPRESSION.id,
                NO_CONFUSING_VOID_EXPRESSION.code,
                NO_CONFUSING_VOID_EXPRESSION.category,
                severity,
                "confusing void expression in value position",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use `void` as a standalone statement or refactor this expression"),
        );
    }
}

/// Return true when one void-like expression is the returned result of a void-returning function.
fn is_void_returning_function_result_position(
    ctx: &LintModuleDirContext<'_>,
    invalid_ancestor_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let is_direct_return_expression = matches!(
        ctx.tree.get(invalid_ancestor_expression_id),
        dir::Expression::Return { value: Some(_) }
    );
    let mut current_child_id = invalid_ancestor_expression_id.into_any();

    loop {
        let Some(parent_node_id) = ctx.tree.get_parent(current_child_id.id) else {
            return false;
        };

        // function boundary
        if parent_node_id.ty == dir::NodeType::Declaration {
            let parent_declaration_id = parent_node_id.into_typed::<dir::Declaration>();
            let parent_declaration = ctx.tree.get(parent_declaration_id);
            let dir::Declaration::Function(declaration) = parent_declaration else {
                return false;
            };

            let Some(body_expression_id) = declaration.body else {
                return false;
            };
            if body_expression_id.id != current_child_id.id {
                return false;
            }

            let function_symbol_id = declaration.symbol.into_global(ctx.module_id());
            let Some(function_type_id) = ctx.types.get_value_type_id(function_symbol_id) else {
                return false;
            };
            let Some(return_type_id) = function_return_type(ctx.types, function_type_id) else {
                return false;
            };
            if !is_void_or_never_type(ctx.types, return_type_id) {
                return false;
            }

            let body_expression = ctx.tree.get(body_expression_id);
            return is_direct_return_expression
                || invalid_ancestor_expression_id == body_expression_id
                    && !matches!(body_expression, dir::Expression::Block(..));
        }

        current_child_id = parent_node_id;
    }
}

impl NodeVisitor for NoConfusingVoidExpressionVisitor<'_, '_> {
    /// Return visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit one expression node.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check the current expression node
        self.check_expression(id);

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Return true when one expression is a candidate void-like expression.
fn is_void_expression_candidate(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    ignore_void_operator: bool,
) -> bool {
    // allow explicit `void` wrappers when configured
    if ignore_void_operator && is_void_unary_expression(ctx.tree, expression_id) {
        return false;
    }

    // keep explicit `void expr` expressions
    if is_void_unary_expression(ctx.tree, expression_id) {
        return true;
    }

    // keep only call-like expressions that type-check to void or never
    if !is_void_or_never_expression(ctx, expression_id) {
        return false;
    }

    let expression = ctx.tree.get(expression_id);
    matches!(
        expression,
        dir::Expression::Call { .. }
            | dir::Expression::Await { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::TaggedTemplateExpression { .. }
    )
}

/// Return true when one expression is typed as void or never.
fn is_void_or_never_expression(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // resolve expression type id from typed DIR
    let Some(type_id) = ctx.expression_type_id(expression_id) else {
        return false;
    };

    // accept void-like expression types
    is_void_or_never_type(ctx.types, type_id)
}

/// Return true when the expression is a `void` unary expression.
fn is_void_unary_expression(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    matches!(
        tree.get(expression_id),
        dir::Expression::Unary {
            operator: UnaryOperator::Void,
            ..
        }
    )
}

/// Return one nearest invalid ancestor when a void-like expression is in value position.
fn invalid_ancestor_expression_id(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // start from outer transparent wrappers around the candidate
    let mut current_expression_id = expression_outer_transparent_ancestor(tree, expression_id);

    // walk ancestor expressions until we hit a valid or invalid boundary
    loop {
        if expression_is_standalone_statement(tree, current_expression_id) {
            return None;
        }

        let Some(parent_expression_id) = expression_parent_id(tree, current_expression_id) else {
            return Some(current_expression_id);
        };

        let parent_expression = tree.get(parent_expression_id);

        // allow non-tail sequence operands
        if is_non_tail_sequence_parent(parent_expression, current_expression_id) {
            return None;
        }

        // recurse through short-circuiting wrappers
        if is_short_circuiting_parent(parent_expression, current_expression_id) {
            current_expression_id =
                expression_outer_transparent_ancestor(tree, parent_expression_id);
            continue;
        }

        return Some(parent_expression_id);
    }
}

/// Return true when the parent is a non-tail sequence wrapper.
fn is_non_tail_sequence_parent(
    parent_expression: &dir::Expression,
    child_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let dir::Expression::SequenceExpression { expressions } = parent_expression else {
        return false;
    };

    expressions
        .last()
        .is_some_and(|last_expression_id| *last_expression_id != child_expression_id)
}

/// Return true when the parent is a short-circuit wrapper around the child.
fn is_short_circuiting_parent(
    parent_expression: &dir::Expression,
    child_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    matches!(
        parent_expression,
        dir::Expression::Binary {
            operator: BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce,
            right,
            ..
        } if *right == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::If {
            then_expression,
            else_expression,
            ..
        } if *then_expression == child_expression_id
            || else_expression.is_some_and(|expression_id| expression_id == child_expression_id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Allow standalone `void` discard statements.
    #[test]
    fn test_allows_standalone_void_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_standalone_void_statement.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

void sideEffect();
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Allow parenthesized standalone `void` discard statements.
    #[test]
    fn test_allows_parenthesized_void_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_parenthesized_void_statement.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

(void sideEffect());
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Flag `void` used in variable initialization.
    #[test]
    fn test_flags_void_in_variable_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_in_variable_initializer.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

const value = void sideEffect();
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag `void` used in return values.
    #[test]
    fn test_flags_void_in_return_value() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_in_return_value.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

function run(): unknown {
    return void sideEffect();
}
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag `void` used as a function argument.
    #[test]
    fn test_flags_void_in_argument_position() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_in_argument_position.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

function consume(value: unknown): void {
    value;
}

consume(void sideEffect());
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Allow explicit `void` wrappers in nested positions when configured.
    #[test]
    fn test_allows_explicit_void_operator_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression).with_options(
            |options| {
                options
                    .correctness
                    .no_confusing_void_expression_ignore_void_operator = true;
            },
        );
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_explicit_void_operator_when_ignored.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

const value = !void sideEffect();
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Allow returned void expressions in void-returning functions when configured.
    #[test]
    fn test_allows_void_return_in_void_returning_function_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression).with_options(
            |options| {
                options
                    .correctness
                    .no_confusing_void_expression_ignore_void_returning_functions = true;
            },
        );
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_void_return_in_void_returning_function_when_ignored.ds",
            r#"
function sideEffect(): void {}

function run(): void {
    return sideEffect();
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Allow void-returning lambda shorthand bodies when configured.
    #[test]
    fn test_allows_void_lambda_body_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression).with_options(
            |options| {
                options
                    .correctness
                    .no_confusing_void_expression_ignore_void_returning_functions = true;
            },
        );
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_void_lambda_body_when_ignored.ts",
            r#"
function sideEffect(): void {}

const run = (): void => sideEffect();
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Keep flagging returned void expressions in non-void functions.
    #[test]
    fn test_flags_void_return_in_non_void_function_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression).with_options(
            |options| {
                options
                    .correctness
                    .no_confusing_void_expression_ignore_void_returning_functions = true;
            },
        );
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_return_in_non_void_function_when_ignored.ds",
            r#"
function sideEffect(): void {}

function run(): unknown {
    return sideEffect();
}
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Keep flagging nested void expressions inside returned value expressions.
    #[test]
    fn test_flags_nested_void_return_expression_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression).with_options(
            |options| {
                options
                    .correctness
                    .no_confusing_void_expression_ignore_void_returning_functions = true;
            },
        );
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_nested_void_return_expression_when_ignored.ds",
            r#"
function sideEffect(): void {}

function run(value: boolean): void {
    return sideEffect() || value;
}
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Allow void in non-tail sequence positions.
    #[test]
    fn test_allows_void_in_non_tail_sequence() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_void_in_non_tail_sequence.ts",
            r#"
function sideEffect(): unknown {
    return 1;
}

const value = (void sideEffect(), 1);
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Flag void in tail sequence positions.
    #[test]
    fn test_flags_void_in_tail_sequence() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_in_tail_sequence.ts",
            r#"
function sideEffect(): unknown {
    return 1;
}

const value = (1, void sideEffect());
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag `void` when nested under passthrough wrappers in value position.
    #[test]
    fn test_flags_void_under_parenthesized_return_wrapper() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_under_parenthesized_return_wrapper.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

function run(): unknown {
    return (void sideEffect());
}
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag cast-wrapped `void` statements because the cast wrapper is noisy.
    #[test]
    fn test_flags_cast_wrapped_void_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_cast_wrapped_void_statement.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

((void sideEffect()) as unknown);
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag cast-wrapped `void` expressions in value-producing positions.
    #[test]
    fn test_flags_cast_wrapped_void_in_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_cast_wrapped_void_in_initializer.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

const value = (void sideEffect()) as unknown;
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Allow void-typed calls used as standalone statements.
    #[test]
    fn test_allows_void_typed_call_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_void_typed_call_statement.ds",
            r#"
function sideEffect(): void {}

sideEffect();
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Flag void-typed calls in value-producing initializers.
    #[test]
    fn test_flags_void_typed_call_in_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_typed_call_in_initializer.ds",
            r#"
function sideEffect(): void {}

const value = sideEffect();
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Allow short-circuiting statement usage for void-typed calls.
    #[test]
    fn test_allows_void_typed_call_in_short_circuit_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_void_typed_call_in_short_circuit_statement.ds",
            r#"
function sideEffect(): void {}

let enabled = true;
enabled && sideEffect();
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Flag short-circuiting value usage when void-typed call contributes to a value.
    #[test]
    fn test_flags_void_typed_call_in_short_circuit_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_typed_call_in_short_circuit_initializer.ds",
            r#"
function sideEffect(): void {}

let enabled = true;
const value = enabled && sideEffect();
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }
}
