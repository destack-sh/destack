use std::collections::HashSet;

use destack_base::StringPool;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::is_strict_boolean_type;
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

const DEFAULT_RELATION_CACHE_KEY: u64 = 0;

declare_lint! {
    /// Prefer nullish coalescing over `||` for nullish defaulting.
    ///
    /// This rule reports `a || b` when the left side can be nullish and
    /// cannot produce other falsy values, so `a ?? b` is clearer and preserves intent.
    #[lint(
        id = "prefer-nullish-coalescing",
        code = "LY081",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub PreferNullishCoalescing,
    "Prefer `??` over `||` for nullish defaults"
}

impl LintRule for PreferNullishCoalescing {
    fn meta(&self) -> &'static LintMeta {
        PreferNullishCoalescing::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect binary logical or expressions
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                right,
                ..
            } = expression
            else {
                continue;
            };

            // skip boolean and control-flow contexts
            if should_skip_expression_context(ctx, expression_id) {
                continue;
            }

            // keep only semantically safe nullish defaulting candidates
            if !left_side_prefers_nullish(ctx, *left) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // report one nullish coalescing suggestion
            let span = ctx.get_span(expression_id);
            let mut diagnostic = LintDiagnostic::new(
                PREFER_NULLISH_COALESCING.id,
                PREFER_NULLISH_COALESCING.code,
                PREFER_NULLISH_COALESCING.category,
                severity,
                "prefer nullish coalescing for defaults",
                ctx.module.file_id,
                span,
            )
            .with_label("use `??` to default only on nullish values");
            if let Some(fix) = make_nullish_fix(ctx, expression_id, *left, *right) {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a safe `||` to `??` fix for one expression.
fn make_nullish_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let expression_span = ctx.get_span(expression_id);
    let left_text = ctx.get_span_text(ctx.get_span(left_id));
    let right_text = ctx.get_span_text(ctx.get_span(right_id));
    if left_text.trim().is_empty() || right_text.trim().is_empty() {
        return None;
    }

    let replacement = format!("{left_text} ?? {right_text}");
    let edits = ctx
        .edit_builder()
        .replace(expression_span, replacement)
        .into_edits();
    Some(LintFix::safe("Replace `||` with `??`").with_edits(edits))
}

/// Return true when this expression should not be linted.
fn should_skip_expression_context(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(parent_id) = ctx.tree.get_parent(expression_id.id) else {
        return false;
    };

    // skip mixed logical chains
    if parent_id.ty == dir::NodeType::Expression {
        let parent = ctx.tree.get(parent_id.into_typed::<dir::Expression>());
        if matches!(
            parent,
            dir::Expression::Binary {
                operator: dir::BinaryOperator::And | dir::BinaryOperator::Or,
                ..
            }
        ) {
            return true;
        }
    }

    // skip condition positions
    expression_is_condition(ctx.tree, expression_id, parent_id)
}

/// Return true when the expression is used as a condition.
fn expression_is_condition(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    parent_id: dir::LocalNodeIdAny,
) -> bool {
    if parent_id.ty == dir::NodeType::Expression {
        let parent = tree.get(parent_id.into_typed::<dir::Expression>());
        match parent {
            dir::Expression::If { condition, .. } => {
                if let dir::IfCondition::Expression { condition } = condition {
                    return *condition == expression_id;
                }
            }
            dir::Expression::Loop {
                condition: Some(condition),
                ..
            } => {
                return *condition == expression_id;
            }
            dir::Expression::For {
                condition: Some(condition),
                ..
            } => {
                return *condition == expression_id;
            }
            _ => {}
        }
    }

    if parent_id.ty == dir::NodeType::MatchCase {
        let match_case = tree.get(parent_id.into_typed::<dir::MatchCase>());
        let selector = match match_case {
            dir::MatchCase::Expression { selector, .. }
            | dir::MatchCase::Block { selector, .. } => selector,
        };
        if let dir::MatchSelector::Pattern {
            guard: Some(guard), ..
        } = selector
        {
            return *guard == expression_id;
        }
    }

    false
}

/// Return true when the left side is a safe candidate for `??`.
fn left_side_prefers_nullish(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(type_id) = ctx.expression_type_id(expression_id) else {
        return false;
    };
    if is_strict_boolean_type(ctx.types, type_id) {
        return false;
    }

    let normalized_type_id = normalize_type(ctx.types, type_id);

    // require maybe-nullish values
    let mut nullish_visited = HashSet::new();
    if !type_is_maybe_nullish(ctx.types, normalized_type_id, &mut nullish_visited) {
        return false;
    }

    // reject candidates where non-nullish falsy values are possible
    let mut falsy_visited = HashSet::new();
    if type_has_non_nullish_falsy(
        ctx.types,
        &ctx.program.strings,
        normalized_type_id,
        &mut falsy_visited,
    ) {
        return false;
    }

    true
}

/// Normalize one type id using flow mode.
fn normalize_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> dir::LocalTypeId {
    types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id)
}

/// Return true when a type can evaluate to a nullish value.
fn type_is_maybe_nullish(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited: &mut HashSet<dir::LocalTypeId>,
) -> bool {
    if !visited.insert(type_id) {
        return false;
    }

    match types.get_type(type_id) {
        dir::Type::TypeLiteral { value } => matches!(
            value,
            dir::TypeLiteral::Null
                | dir::TypeLiteral::Undefined
                | dir::TypeLiteral::Void
                | dir::TypeLiteral::Any
                | dir::TypeLiteral::Infer
                | dir::TypeLiteral::Unknown
        ),
        dir::Type::Value { value } => type_is_maybe_nullish(types, *value, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => type_is_maybe_nullish(types, *right, visited),
        dir::Type::Reference { symbol, .. } => {
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return type_is_maybe_nullish(types, instance_type_id, visited);
            }
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return type_is_maybe_nullish(types, value_type_id, visited);
            }

            false
        }
        dir::Type::Union { elements } => elements
            .iter()
            .any(|element| type_is_maybe_nullish(types, *element, visited)),
        dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| type_is_maybe_nullish(types, *element, visited)),
        dir::Type::InferVar { .. }
        | dir::Type::Conditional { .. }
        | dir::Type::Mapped { .. }
        | dir::Type::Index { .. }
        | dir::Type::TemplateLiteral { .. }
        | dir::Type::Import { .. }
        | dir::Type::Infer { .. }
        | dir::Type::Predicate { .. }
        | dir::Type::Unary { .. }
        | dir::Type::Binary { .. }
        | dir::Type::Error
        | dir::Type::Unevaluated(_) => true,
        dir::Type::This => false,
        dir::Type::Array { .. }
        | dir::Type::ArraySized { .. }
        | dir::Type::Tuple { .. }
        | dir::Type::Object { .. }
        | dir::Type::Function { .. } => false,
    }
}

/// Return true when a type can evaluate to a non-nullish falsy value.
fn type_has_non_nullish_falsy(
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
    visited: &mut HashSet<dir::LocalTypeId>,
) -> bool {
    if !visited.insert(type_id) {
        return false;
    }

    match types.get_type(type_id) {
        dir::Type::TypeLiteral { value } => match value {
            dir::TypeLiteral::Null | dir::TypeLiteral::Undefined | dir::TypeLiteral::Void => false,
            dir::TypeLiteral::Never => false,
            dir::TypeLiteral::Any | dir::TypeLiteral::Infer | dir::TypeLiteral::Unknown => true,
            dir::TypeLiteral::Object => false,
            dir::TypeLiteral::Primitive(primitive) => matches!(
                primitive,
                dir::PrimitiveType::Boolean
                    | dir::PrimitiveType::Number
                    | dir::PrimitiveType::Bigint
                    | dir::PrimitiveType::String
                    | dir::PrimitiveType::Int(_)
                    | dir::PrimitiveType::Float(_)
            ),
            dir::TypeLiteral::ScalarLiteral(literal) => match literal {
                dir::ScalarLiteral::Boolean(false) => true,
                dir::ScalarLiteral::Boolean(true) => false,
                dir::ScalarLiteral::Integer(value) => *value == 0,
                dir::ScalarLiteral::Bigint(value) => *value == 0,
                dir::ScalarLiteral::Float(value) => *value == 0.0,
                dir::ScalarLiteral::String(value) => strings.get(*value).is_empty(),
                dir::ScalarLiteral::Character(_) | dir::ScalarLiteral::RegexString { .. } => true,
            },
            dir::TypeLiteral::Intrinsic(_) => true,
        },
        dir::Type::Value { value } => type_has_non_nullish_falsy(types, strings, *value, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            type_has_non_nullish_falsy(types, strings, *right, visited)
        }
        dir::Type::Reference { symbol, .. } => {
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return type_has_non_nullish_falsy(types, strings, instance_type_id, visited);
            }
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return type_has_non_nullish_falsy(types, strings, value_type_id, visited);
            }

            true
        }
        dir::Type::Union { elements } => elements
            .iter()
            .any(|element| type_has_non_nullish_falsy(types, strings, *element, visited)),
        dir::Type::Intersection { elements } => elements
            .iter()
            .any(|element| type_has_non_nullish_falsy(types, strings, *element, visited)),
        dir::Type::Array { .. }
        | dir::Type::ArraySized { .. }
        | dir::Type::Tuple { .. }
        | dir::Type::Object { .. }
        | dir::Type::Function { .. } => false,
        dir::Type::InferVar { .. }
        | dir::Type::This
        | dir::Type::Unevaluated(_)
        | dir::Type::Conditional { .. }
        | dir::Type::Mapped { .. }
        | dir::Type::Index { .. }
        | dir::Type::TemplateLiteral { .. }
        | dir::Type::Import { .. }
        | dir::Type::Infer { .. }
        | dir::Type::Predicate { .. }
        | dir::Type::Unary { .. }
        | dir::Type::Binary { .. }
        | dir::Type::Error => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report `||` defaulting when the left side is object or null.
    #[test]
    fn test_flags_object_or_null_defaulting() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_flags_object_or_null_defaulting.ds",
            r#"
let profile: { id: int32 } | null = null;
let selected = profile || { id: 1 };
"#,
        );

        test.result(diagnostics)
            .assert_lint("prefer-nullish-coalescing")
            .assert_has_fix("prefer-nullish-coalescing")
            .assert_safe_fixed(
                r#"
let profile: { id: int32 } | null = null;
let selected = profile ?? { id: 1 };
"#,
            );
    }

    /// Allow `||` when non-nullish falsy string values are possible.
    #[test]
    fn test_allows_string_or_null_defaulting() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_string_or_null_defaulting.ds",
            r#"
let title: string | null = "";
let selected = title || "fallback";
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Allow `||` when non-nullish falsy numeric values are possible.
    #[test]
    fn test_allows_number_or_null_defaulting() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_number_or_null_defaulting.ds",
            r#"
let count: int32 | null = 0;
let selected = count || 1;
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Allow `||` in condition positions.
    #[test]
    fn test_allows_condition_context() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_condition_context.ds",
            r#"
let condition: { ok: boolean } | null = null;
if (condition || { ok: true }) {
}
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Allow `||` when the left side is never nullish.
    #[test]
    fn test_allows_never_nullish_left_side() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_never_nullish_left_side.ds",
            r#"
let profile: { id: int32 } = { id: 1 };
let selected = profile || { id: 2 };
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }
}
