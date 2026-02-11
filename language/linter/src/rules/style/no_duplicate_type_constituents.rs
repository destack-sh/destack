use std::collections::HashMap;

use destack_ast as ast;
use destack_source::LabeledSpan;
use destack_workspace::LintSeverity;

use crate::rules::common::ExpressionDuplicateTracker;
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate constituents in union and intersection types.
    ///
    /// Duplicate constituents add noise and do not change type meaning.
    /// Example: `A | A | B` should be simplified to `A | B`.
    #[lint(
        id = "no-duplicate-type-constituents",
        code = "LY072",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoDuplicateTypeConstituents,
    "Disallow duplicate constituents in union and intersection types"
}

impl LintRule for NoDuplicateTypeConstituents {
    fn meta(&self) -> &'static crate::LintMeta {
        NoDuplicateTypeConstituents::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // inspect top-level type union/intersection chains
        for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let ast::Expression::Binary { operator, .. } = expression else {
                continue;
            };

            let is_type_constituent_operator = matches!(
                operator,
                ast::BinaryOperator::ElementwiseOr | ast::BinaryOperator::ElementwiseAnd
            );
            if !is_type_constituent_operator {
                continue;
            }
            if is_nested_same_operator(ctx, expression_id, *operator) {
                continue;
            }
            if !expression_is_in_type_position(ctx, expression_id) {
                continue;
            }

            report_duplicate_constituents(ctx, meta, expression_id, *operator);
        }
    }
}

/// Report duplicate constituents for one top-level type chain.
fn report_duplicate_constituents(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &crate::LintMeta,
    expression_id: ast::LocalNodeId<ast::Expression>,
    operator: ast::BinaryOperator,
) {
    let mut constituents = Vec::new();
    flatten_constituents(ctx, expression_id, operator, &mut constituents);
    if constituents.len() < 2 {
        return;
    }

    // find duplicate positions by structural expression equality
    let mut duplicate_tracker = ExpressionDuplicateTracker::new();
    let mut first_index_by_expression_id = HashMap::<u32, usize>::new();
    let mut duplicate_pairs = Vec::<(usize, usize)>::new();
    for (index, constituent_id) in constituents.iter().enumerate() {
        if let Some(first_expression_id) =
            duplicate_tracker.find_duplicate_or_insert(ctx, *constituent_id)
        {
            let Some(first_index) = first_index_by_expression_id
                .get(&first_expression_id.id)
                .copied()
            else {
                continue;
            };
            duplicate_pairs.push((index, first_index));
        } else {
            first_index_by_expression_id.insert(constituent_id.id, index);
        }
    }

    if duplicate_pairs.is_empty() {
        return;
    }

    // build one deduplicated replacement for safe auto-fix
    let deduplicated_replacement =
        deduplicated_constituent_replacement(ctx, &constituents, operator, &duplicate_pairs);

    for (duplicate_position, (duplicate_index, first_index)) in duplicate_pairs.iter().enumerate() {
        let duplicate_constituent_id = constituents[*duplicate_index];
        let first_constituent_id = constituents[*first_index];

        let severity = ctx.get_effective_severity(meta, duplicate_constituent_id);
        if !severity.is_enabled() {
            continue;
        }

        let duplicate_span = ctx.tree.get_span(duplicate_constituent_id);
        let first_span = ctx.tree.get_span(first_constituent_id);
        let mut diagnostic = LintDiagnostic::new(
            NO_DUPLICATE_TYPE_CONSTITUENTS.id,
            NO_DUPLICATE_TYPE_CONSTITUENTS.code,
            NO_DUPLICATE_TYPE_CONSTITUENTS.category,
            severity,
            "duplicate type constituent",
            ctx.module.file_id,
            duplicate_span,
        )
        .with_label("this constituent duplicates an earlier constituent")
        .with_secondary(LabeledSpan::new(first_span, "first occurrence is here"));

        // attach one replacement fix once to avoid overlapping edits
        if duplicate_position == 0
            && ctx.compute_fixes
            && let Some(replacement) = deduplicated_replacement.as_ref()
        {
            let chain_span = ctx.tree.get_span(expression_id);
            let edits = ctx
                .edit_builder()
                .replace(chain_span, replacement)
                .into_edits();
            let fix = LintFix::safe("Remove duplicate type constituents and keep unique members")
                .with_edits(edits);
            diagnostic = diagnostic.with_fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// Return true when this expression is nested under the same chain operator.
fn is_nested_same_operator(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    operator: ast::BinaryOperator,
) -> bool {
    let mut current_node_id = expression_id.id;

    loop {
        let Some(parent_node_id) = ctx.parents.get_by_id(current_node_id) else {
            return false;
        };
        if ctx.tree.get_node_type(parent_node_id) != ast::NodeType::Expression {
            return false;
        }

        let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_node_id);
        let parent_expression = ctx.tree.get(parent_expression_id);
        match parent_expression {
            ast::Expression::Parenthesized { expression } if expression.id == current_node_id => {
                current_node_id = parent_node_id;
            }
            ast::Expression::Binary {
                operator: parent_operator,
                ..
            } => return *parent_operator == operator,
            _ => return false,
        }
    }
}

/// Flatten all constituents from one chain expression.
fn flatten_constituents(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    operator: ast::BinaryOperator,
    constituents: &mut Vec<ast::LocalNodeId<ast::Expression>>,
) {
    let expression_id = unwrap_parenthesized_expression(ctx, expression_id);
    let expression = ctx.tree.get(expression_id);
    if let ast::Expression::Binary {
        left,
        operator: child_operator,
        right,
    } = expression
        && *child_operator == operator
    {
        flatten_constituents(ctx, *left, operator, constituents);
        flatten_constituents(ctx, *right, operator, constituents);
        return;
    }

    constituents.push(expression_id);
}

/// Return one expression id with enclosing parentheses removed.
fn unwrap_parenthesized_expression(
    ctx: &LintModuleAstContext<'_>,
    mut expression_id: ast::LocalNodeId<ast::Expression>,
) -> ast::LocalNodeId<ast::Expression> {
    loop {
        let expression = ctx.tree.get(expression_id);
        let ast::Expression::Parenthesized { expression } = expression else {
            return expression_id;
        };
        expression_id = *expression;
    }
}

/// Build one deduplicated replacement expression text.
fn deduplicated_constituent_replacement(
    ctx: &LintModuleAstContext<'_>,
    constituents: &[ast::LocalNodeId<ast::Expression>],
    operator: ast::BinaryOperator,
    duplicate_pairs: &[(usize, usize)],
) -> Option<String> {
    if duplicate_pairs.is_empty() {
        return None;
    }

    let mut duplicate_index_flags = vec![false; constituents.len()];
    for (duplicate_index, _) in duplicate_pairs {
        duplicate_index_flags[*duplicate_index] = true;
    }

    let mut unique_texts = Vec::<String>::new();

    for (index, constituent_id) in constituents.iter().enumerate() {
        if duplicate_index_flags[index] {
            continue;
        }

        let constituent_span = ctx.tree.get_span(*constituent_id);
        let constituent_text = ctx.get_span_text(constituent_span).trim();
        if constituent_text.is_empty() {
            continue;
        }
        unique_texts.push(constituent_text.to_string());
    }

    if unique_texts.is_empty() {
        return None;
    }

    let separator = match operator {
        ast::BinaryOperator::ElementwiseOr => " | ",
        ast::BinaryOperator::ElementwiseAnd => " & ",
        _ => return None,
    };
    Some(unique_texts.join(separator))
}

/// Return true when one expression belongs to any known type position.
fn expression_is_in_type_position(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let mut current_node_id = expression_id.id;

    loop {
        let Some(parent_node_id) = ctx.parents.get_by_id(current_node_id) else {
            return false;
        };
        let parent_node_type = ctx.tree.get_node_type(parent_node_id);
        match parent_node_type {
            ast::NodeType::Expression => {
                let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_node_id);
                let parent_expression = ctx.tree.get(parent_expression_id);
                if expression_is_type_slot_in_parent_expression(parent_expression, current_node_id)
                {
                    return true;
                }
            }
            ast::NodeType::Declarator => {
                let parent_declarator_id = ast::LocalNodeId::<ast::Declarator>::new(parent_node_id);
                let parent_declarator = ctx.tree.get(parent_declarator_id);
                if parent_declarator
                    .ty
                    .is_some_and(|type_expression_id| type_expression_id.id == current_node_id)
                {
                    return true;
                }
            }
            ast::NodeType::Parameter => {
                let parent_parameter_id = ast::LocalNodeId::<ast::Parameter>::new(parent_node_id);
                let parent_parameter = ctx.tree.get(parent_parameter_id);
                if parameter_type_slot_contains_expression(parent_parameter, current_node_id) {
                    return true;
                }
            }
            ast::NodeType::Declaration => {
                let parent_declaration_id =
                    ast::LocalNodeId::<ast::Declaration>::new(parent_node_id);
                let parent_declaration = ctx.tree.get(parent_declaration_id);
                if declaration_type_slot_contains_expression(parent_declaration, current_node_id) {
                    return true;
                }
            }
            ast::NodeType::Member => {
                let parent_member_id = ast::LocalNodeId::<ast::Member>::new(parent_node_id);
                let parent_member = ctx.tree.get(parent_member_id);
                if member_type_slot_contains_expression(parent_member, current_node_id) {
                    return true;
                }
            }
            ast::NodeType::WhereClause => {
                let parent_where_clause_id =
                    ast::LocalNodeId::<ast::WhereClause>::new(parent_node_id);
                let parent_where_clause = ctx.tree.get(parent_where_clause_id);
                if parent_where_clause.right.id == current_node_id {
                    return true;
                }
            }
            _ => {}
        }

        current_node_id = parent_node_id;
    }
}

/// Return true when this child expression is in one parent expression type slot.
fn expression_is_type_slot_in_parent_expression(
    parent_expression: &ast::Expression,
    child_id: u32,
) -> bool {
    match parent_expression {
        ast::Expression::TypeUnary { right, .. } => right.id == child_id,
        ast::Expression::TypeBinary { left, right, .. } => {
            left.id == child_id || right.id == child_id
        }
        ast::Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            left.id == child_id
                || right.id == child_id
                || then_type.id == child_id
                || else_type.id == child_id
        }
        ast::Expression::TypeMapped {
            parameter, value, ..
        } => {
            parameter.constraint.id == child_id
                || parameter
                    .key_remap
                    .is_some_and(|key_remap_id| key_remap_id.id == child_id)
                || value.id == child_id
        }
        ast::Expression::TypeIndex { left, index } => left.id == child_id || index.id == child_id,
        ast::Expression::TypeTemplateLiteral { spans, .. } => {
            spans.iter().any(|span_id| span_id.id == child_id)
        }
        ast::Expression::TypeImport { target, .. } => target.id == child_id,
        ast::Expression::TypeInfer {
            constraint: Some(constraint),
            ..
        } => constraint.id == child_id,
        ast::Expression::TypePredicate {
            target: Some(target),
            ..
        } => target.id == child_id,
        _ => false,
    }
}

/// Return true when one parameter type slot points at this expression.
fn parameter_type_slot_contains_expression(parameter: &ast::Parameter, expression_id: u32) -> bool {
    match parameter {
        ast::Parameter::Named { ty, .. }
        | ast::Parameter::Pattern { ty, .. }
        | ast::Parameter::VariadicNamed { ty, .. }
        | ast::Parameter::VariadicPattern { ty, .. } => {
            ty.is_some_and(|type_expression_id| type_expression_id.id == expression_id)
        }
    }
}

/// Return true when one declaration type slot points at this expression.
fn declaration_type_slot_contains_expression(
    declaration: &ast::Declaration,
    expression_id: u32,
) -> bool {
    match declaration {
        ast::Declaration::Type { value, .. } => value.id == expression_id,
        ast::Declaration::Function { signature, .. } => signature
            .return_type
            .is_some_and(|return_type_id| return_type_id.id == expression_id),
        ast::Declaration::Extension { target_type, .. } => target_type.id == expression_id,
        ast::Declaration::Struct { heritage, .. }
        | ast::Declaration::Class { heritage, .. }
        | ast::Declaration::Interface { heritage, .. }
        | ast::Declaration::Enum { heritage, .. } => {
            heritage
                .extends_types
                .as_ref()
                .is_some_and(|types| types.iter().any(|type_id| type_id.id == expression_id))
                || heritage
                    .implements_types
                    .as_ref()
                    .is_some_and(|types| types.iter().any(|type_id| type_id.id == expression_id))
        }
        _ => false,
    }
}

/// Return true when one member type slot points at this expression.
fn member_type_slot_contains_expression(member: &ast::Member, expression_id: u32) -> bool {
    match member {
        ast::Member::Type { ty, value, .. } => {
            ty.is_some_and(|type_expression_id| type_expression_id.id == expression_id)
                || value.is_some_and(|value_expression_id| value_expression_id.id == expression_id)
        }
        ast::Member::ComptimeConst { ty, .. } => {
            ty.is_some_and(|type_expression_id| type_expression_id.id == expression_id)
        }
        ast::Member::Field { value, .. } => {
            value.is_some_and(|type_expression_id| type_expression_id.id == expression_id)
        }
        ast::Member::Embed { value, .. } => value.id == expression_id,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag duplicate union constituents.
    #[test]
    fn test_flags_duplicate_union_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_ast(
            "no_duplicate_type_constituents/test_flags_duplicate_union_constituents.ds",
            r#"
type Value = string | number | string;
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }

    /// Flag duplicate intersection constituents.
    #[test]
    fn test_flags_duplicate_intersection_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_ast(
            "no_duplicate_type_constituents/test_flags_duplicate_intersection_constituents.ds",
            r#"
type Value = Readable & Writable & Readable;
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }

    /// Allow unique type constituents.
    #[test]
    fn test_allows_unique_type_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_ast(
            "no_duplicate_type_constituents/test_allows_unique_type_constituents.ds",
            r#"
type Value = string | number | boolean;
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-type-constituents");
    }

    /// Ignore value-level elementwise expressions.
    #[test]
    fn test_ignores_value_level_elementwise_expressions() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_ast(
            "no_duplicate_type_constituents/test_ignores_value_level_elementwise_expressions.ds",
            r#"
let result = a | b | a;
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-type-constituents");
    }

    /// Fix duplicate union constituents by preserving first occurrence order.
    #[test]
    fn test_fix_deduplicates_union_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_ast(
            "no_duplicate_type_constituents/test_fix_deduplicates_union_constituents.ds",
            r#"
type Value = string | number | string | boolean | number;
"#,
        );
        test.result(result)
            .assert_lint_count("no-duplicate-type-constituents", 2)
            .assert_safe_fixed(
                r#"
type Value = string | number | boolean;
"#,
            );
    }

    /// Fix duplicate intersection constituents by preserving first occurrence order.
    #[test]
    fn test_fix_deduplicates_intersection_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_ast(
            "no_duplicate_type_constituents/test_fix_deduplicates_intersection_constituents.ds",
            r#"
type Value = Readable & Writable & Readable & Writable;
"#,
        );
        test.result(result)
            .assert_lint_count("no-duplicate-type-constituents", 2)
            .assert_safe_fixed(
                r#"
type Value = Readable & Writable;
"#,
            );
    }

    /// Detect duplicate constituents in function return type positions.
    #[test]
    fn test_flags_duplicate_constituents_in_function_return_type() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_ast(
            "no_duplicate_type_constituents/test_flags_duplicate_constituents_in_function_return_type.ds",
            r#"
function value(): string | string {
    return "ok";
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }

    /// Detect duplicate string literal constituents independent of source quote style.
    #[test]
    fn test_flags_duplicate_string_literal_constituents_with_different_quote_style() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_ast(
            "no_duplicate_type_constituents/test_flags_duplicate_string_literal_constituents_with_different_quote_style.ds",
            r#"
type Value = "ok" | 'ok' | "next";
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }

    /// Detect duplicate constituents nested inside grouped chains.
    #[test]
    fn test_flags_nested_duplicate_constituent_chain() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_ast(
            "no_duplicate_type_constituents/test_flags_nested_duplicate_constituent_chain.ds",
            r#"
type Value = (Alpha | Beta) | Alpha;
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }
}
