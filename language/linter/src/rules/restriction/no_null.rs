use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_path_segments, expression_static_property_access_source_form,
};
use crate::{LintAstContext, LintFix, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow the use of `null`.
    ///
    /// Using only `undefined` for absent values leads to more consistent code.
    /// Consider using `undefined` instead of `null`.
    #[lint(
        id = "no-null",
        code = "LR019",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Off,
        stability = Stable
    )]
    pub NoNull,
    "Disallow null"
}

impl LintRule for NoNull {
    fn meta(&self) -> &'static LintMeta {
        NoNull::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if matches!(
                expression,
                ast::Expression::ScalarLiteral(ast::ScalarLiteral::Null)
            ) {
                if is_allowed_null_usage(ctx, node_id) {
                    continue;
                }

                // resolve effective lint severity
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let span = ctx.tree.get_span(node_id);
                let mut diagnostic = LintReport::new(
                    NO_NULL.id,
                    NO_NULL.code,
                    NO_NULL.category,
                    severity,
                    "use of `null`",
                    span,
                )
                .label("use `undefined` instead");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes {
                    let edits = ctx.edit_builder().replace(span, "undefined").into_edits();
                    let fix =
                        LintFix::r#unsafe("Replace `null` with `undefined`").with_edits(edits);
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }

        // inspect candidate type expressions
        for node_id in ctx.tree.iter_nodes::<ast::TypeExpression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(
                expression,
                ast::TypeExpression::Literal {
                    value: ast::TypeLiteral::Null,
                }
            ) {
                continue;
            }
            if is_allowed_type_null_usage(ctx, node_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_NULL.id,
                NO_NULL.code,
                NO_NULL.category,
                severity,
                "use of `null`",
                span,
            )
            .label("use `undefined` instead");

            if ctx.compute_fixes {
                let edits = ctx.edit_builder().replace(span, "undefined").into_edits();
                let fix = LintFix::r#unsafe("Replace `null` with `undefined`").with_edits(edits);
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when one type-space null usage should be allowed.
fn is_allowed_type_null_usage(
    _ctx: &LintAstContext<'_>,
    _null_expression_id: ast::LocalNodeId<ast::TypeExpression>,
) -> bool {
    false
}

/// Return true when one null usage should be allowed for runtime semantics.
fn is_allowed_null_usage(
    ctx: &LintAstContext<'_>,
    null_expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // allow strict comparisons when configured
    if !ctx.options.restriction.check_strict_null_equality
        && null_is_in_strict_equality_comparison(ctx, null_expression_id)
    {
        return true;
    }

    // resolve one call argument usage for this null literal
    let Some((callee_expression_id, argument_index)) =
        find_call_argument_usage(ctx, null_expression_id)
    else {
        return false;
    };

    // allow Object.create(null)
    if argument_index == 0 && expression_is_object_create(ctx, callee_expression_id) {
        return true;
    }

    // allow useRef(null) and React.useRef(null)
    if argument_index == 0 && expression_is_use_ref(ctx, callee_expression_id) {
        return true;
    }

    // allow node.insertBefore(child, null)
    argument_index == 1 && expression_is_insert_before(ctx, callee_expression_id)
}

/// Return true when one null literal participates in a strict equality comparison.
fn null_is_in_strict_equality_comparison(
    ctx: &LintAstContext<'_>,
    null_expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let mut current_expression_id = null_expression_id;

    loop {
        let Some(parent_id) = ctx.parents.get(current_expression_id) else {
            return false;
        };
        if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
            return false;
        }

        let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
        let parent_expression = ctx.tree.get(parent_expression_id);
        match parent_expression {
            ast::Expression::Parenthesized { expression }
                if *expression == current_expression_id =>
            {
                current_expression_id = parent_expression_id;
            }
            ast::Expression::Binary {
                left,
                operator,
                right,
            } if (*left == current_expression_id || *right == current_expression_id)
                && matches!(
                    operator,
                    ast::BinaryOperator::EqualStrict | ast::BinaryOperator::NotEqualStrict
                ) =>
            {
                return true;
            }
            _ => return false,
        }
    }
}

/// Return one call callee and argument index for one null argument usage.
fn find_call_argument_usage(
    ctx: &LintAstContext<'_>,
    null_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<(ast::LocalNodeId<ast::Expression>, usize)> {
    // start from the null expression
    let mut current_expression_id = null_expression_id;

    // walk through parenthesized wrappers before argument matching
    loop {
        let parent_id = ctx.parents.get(current_expression_id)?;
        if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
            break;
        }

        // resolve parent expression id
        let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
        let parent_expression = ctx.tree.get(parent_expression_id);
        let ast::Expression::Parenthesized { expression } = parent_expression else {
            break;
        };
        if *expression != current_expression_id {
            break;
        }

        current_expression_id = parent_expression_id;
    }

    // require one argument parent
    let argument_node_id = ctx.parents.get(current_expression_id)?;
    if ctx.tree.get_node_type(argument_node_id) != ast::NodeType::Argument {
        return None;
    }
    let argument_id = ast::LocalNodeId::<ast::Argument>::new(argument_node_id);

    // require one call expression parent
    let call_node_id = ctx.parents.get(argument_id)?;
    if ctx.tree.get_node_type(call_node_id) != ast::NodeType::Expression {
        return None;
    }
    let call_expression_id = ast::LocalNodeId::<ast::Expression>::new(call_node_id);
    let call_expression = ctx.tree.get(call_expression_id);
    let ast::Expression::Call {
        left, arguments, ..
    } = call_expression
    else {
        return None;
    };

    arguments
        .iter()
        .position(|current_argument_id| *current_argument_id == argument_id)
        .map(|argument_index| (*left, argument_index))
}

/// Return true when one expression resolves to Object.create.
fn expression_is_object_create(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let object_name = ctx.string_id("Object");
    let create_name = ctx.string_id("create");

    // match path calls like Object.create(...)
    if let Some(path_segments) = expression_path_segments(ctx.tree, expression_id)
        && path_segments.as_slice() == [object_name, create_name]
    {
        return true;
    }

    // match computed member calls like Object["create"](...)
    let Some((receiver_id, property_name)) =
        expression_static_property_access_source_form(ctx.tree, expression_id)
    else {
        return false;
    };
    if property_name != create_name {
        return false;
    }

    matches!(
        expression_path_segments(ctx.tree, receiver_id),
        Some(segments) if segments.len() == 1 && segments[0] == object_name
    )
}

/// Return true when one expression resolves to useRef.
fn expression_is_use_ref(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let use_ref_name = ctx.string_id("useRef");
    let react_name = ctx.string_id("React");

    // match path calls like useRef(...) and React.useRef(...)
    if let Some(path_segments) = expression_path_segments(ctx.tree, expression_id) {
        if path_segments.as_slice() == [use_ref_name] {
            return true;
        }
        if path_segments.as_slice() == [react_name, use_ref_name] {
            return true;
        }
    }

    // match computed member calls like React["useRef"](...)
    let Some((receiver_id, property_name)) =
        expression_static_property_access_source_form(ctx.tree, expression_id)
    else {
        return false;
    };
    if property_name != use_ref_name {
        return false;
    }

    matches!(
        expression_path_segments(ctx.tree, receiver_id),
        Some(segments) if segments.len() == 1 && segments[0] == react_name
    )
}

/// Return true when one expression resolves to an insertBefore member call.
fn expression_is_insert_before(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let insert_before_name = ctx.string_id("insertBefore");

    // match path calls like parent.insertBefore(...)
    if let Some(path_segments) = expression_path_segments(ctx.tree, expression_id)
        && path_segments.len() >= 2
        && path_segments.last().copied() == Some(insert_before_name)
    {
        return true;
    }

    // match computed member calls like parent["insertBefore"](...)
    let Some((_, property_name)) =
        expression_static_property_access_source_form(ctx.tree, expression_id)
    else {
        return false;
    };

    property_name == insert_before_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_null_literal() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast("no_null/test_detects_null_literal.ts", "const x = null;");
        test.result(result)
            .assert_lint("no-null")
            .assert_has_fix("no-null");
    }

    #[test]
    fn test_detects_null_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_detects_null_comparison.ts",
            "if (x === null) {}",
        );
        test.result(result).assert_lint("no-null");
    }

    #[test]
    fn test_allows_strict_null_comparison_when_disabled() {
        let test = TestProgram::for_rule_without_prelude(NoNull).with_options(|options| {
            options.restriction.check_strict_null_equality = false;
        });
        let result = test.lint_ast(
            "no_null/test_allows_strict_null_comparison_when_disabled.ts",
            "if (value === null) {}",
        );
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_still_detects_loose_null_comparison_when_strict_equality_disabled() {
        let test = TestProgram::for_rule_without_prelude(NoNull).with_options(|options| {
            options.restriction.check_strict_null_equality = false;
        });
        let result = test.lint_ast(
            "no_null/test_still_detects_loose_null_comparison_when_strict_equality_disabled.ts",
            "if (value == null) {}",
        );
        test.result(result).assert_lint("no-null");
    }

    #[test]
    fn test_allows_undefined() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast("no_null/test_allows_undefined.ts", "const x = undefined;");
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_allows_optional() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_allows_optional.ts",
            "const x: string | undefined = undefined;",
        );
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_fix_rewrites_null_literal() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_fix_rewrites_null_literal.ts",
            r#"
const value = null;
"#,
        );
        test.result(result)
            .assert_lint("no-null")
            .assert_unsafe_fixed(
                r#"
const value = undefined;
"#,
            );
    }

    #[test]
    fn test_mutation_fix_rewrites_null_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_mutation_fix_rewrites_null_comparison.ts",
            r#"
if (value === null) {}
"#,
        );
        test.result(result)
            .assert_lint("no-null")
            .assert_unsafe_fixed(
                r#"
if (value === undefined) {
}
"#,
            );
    }

    #[test]
    fn test_allows_object_create_null() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_allows_object_create_null.ts",
            "const value = Object.create(null);",
        );
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_allows_use_ref_null() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_allows_use_ref_null.ts",
            "const ref = useRef(null);",
        );
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_allows_react_use_ref_null() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_allows_react_use_ref_null.ts",
            "const ref = React.useRef(null);",
        );
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_allows_insert_before_null() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_allows_insert_before_null.ts",
            "parent.insertBefore(child, null);",
        );
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_allows_parenthesized_object_create_null() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_allows_parenthesized_object_create_null.ts",
            "const value = Object.create((null));",
        );
        test.result(result).assert_no_lint("no-null");
    }

    #[test]
    fn test_allows_parenthesized_insert_before_null() {
        let test = TestProgram::for_rule_without_prelude(NoNull);
        let result = test.lint_ast(
            "no_null/test_allows_parenthesized_insert_before_null.ts",
            "parent.insertBefore(child, (null));",
        );
        test.result(result).assert_no_lint("no-null");
    }
}
