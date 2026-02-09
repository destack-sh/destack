use std::collections::HashSet;

use destack_base::StringPool;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression, walk_match_case};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

const DEFAULT_RELATION_CACHE_KEY: u64 = 0;

declare_lint! {
    /// Disallow conditions that are always truthy, always falsy, or nullish-fixed.
    ///
    /// This catches conditions that cannot change outcome based on their
    /// static type and nullish coalescing where the left side is known.
    #[lint(
        id = "no-unnecessary-condition",
        code = "LC071",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoUnnecessaryCondition,
    "Disallow conditions that are always truthy, always falsy, or nullish-fixed"
}

impl LintRule for NoUnnecessaryCondition {
    fn meta(&self) -> &'static LintMeta {
        NoUnnecessaryCondition::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = UnnecessaryConditionVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor for unnecessary condition checks.
struct UnnecessaryConditionVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> UnnecessaryConditionVisitor<'a, 'b> {
    /// Build a visitor for unnecessary condition checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one condition expression.
    fn check_condition(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        context_label: &'static str,
    ) {
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return;
        };
        let truthiness = type_truthiness(self.ctx.types, &self.ctx.program.strings, type_id);

        let (message, label) = match truthiness {
            Truthiness::AlwaysTruthy => (
                format!("{context_label} is always truthy"),
                "this condition always evaluates to true",
            ),
            Truthiness::AlwaysFalsy => (
                format!("{context_label} is always falsy"),
                "this condition always evaluates to false",
            ),
            Truthiness::Unknown => return,
        };

        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_UNNECESSARY_CONDITION.id,
                NO_UNNECESSARY_CONDITION.code,
                NO_UNNECESSARY_CONDITION.category,
                severity,
                message,
                self.ctx.module.file_id,
                span,
            )
            .with_label(label),
        );
    }

    /// Check one nullish coalescing expression.
    fn check_coalesce(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
    ) {
        let Some(type_id) = self.ctx.expression_type_id(left_id) else {
            return;
        };
        let nullishness = type_nullishness(self.ctx.types, type_id);

        let (message, label) = match nullishness {
            Nullishness::NeverNullish => (
                "left side of ?? is never nullish".to_string(),
                "the right side is unreachable",
            ),
            Nullishness::AlwaysNullish => (
                "left side of ?? is always nullish".to_string(),
                "the left side is unreachable",
            ),
            Nullishness::MaybeNullish => return,
        };

        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_UNNECESSARY_CONDITION.id,
                NO_UNNECESSARY_CONDITION.code,
                NO_UNNECESSARY_CONDITION.category,
                severity,
                message,
                self.ctx.module.file_id,
                span,
            )
            .with_label(label),
        );
    }

    /// Check one logical short-circuit expression.
    fn check_logical(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
    ) {
        let Some(type_id) = self.ctx.expression_type_id(left_id) else {
            return;
        };
        let truthiness = type_truthiness(self.ctx.types, &self.ctx.program.strings, type_id);

        let (message, label) = match (operator, truthiness) {
            (dir::BinaryOperator::And, Truthiness::AlwaysFalsy) => (
                "left side of && is always falsy".to_string(),
                "the right side is unreachable",
            ),
            (dir::BinaryOperator::And, Truthiness::AlwaysTruthy) => (
                "left side of && is always truthy".to_string(),
                "the left side is redundant",
            ),
            (dir::BinaryOperator::Or, Truthiness::AlwaysTruthy) => (
                "left side of || is always truthy".to_string(),
                "the right side is unreachable",
            ),
            (dir::BinaryOperator::Or, Truthiness::AlwaysFalsy) => (
                "left side of || is always falsy".to_string(),
                "the left side is redundant",
            ),
            _ => return,
        };

        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_UNNECESSARY_CONDITION.id,
                NO_UNNECESSARY_CONDITION.code,
                NO_UNNECESSARY_CONDITION.category,
                severity,
                message,
                self.ctx.module.file_id,
                span,
            )
            .with_label(label),
        );
    }
}

impl NodeVisitor for UnnecessaryConditionVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::If { condition, .. } => {
                if let dir::IfCondition::Expression { condition } = condition {
                    self.check_condition(*condition, "if condition");
                }
            }
            dir::Expression::Loop {
                condition: Some(condition),
                ..
            } => {
                self.check_condition(*condition, "loop condition");
            }
            dir::Expression::For {
                condition: Some(condition),
                ..
            } => {
                self.check_condition(*condition, "for condition");
            }
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Coalesce,
                ..
            } => {
                self.check_coalesce(id, *left);
            }
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::And,
                ..
            } => {
                self.check_logical(id, *left, dir::BinaryOperator::And);
            }
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                ..
            } => {
                self.check_logical(id, *left, dir::BinaryOperator::Or);
            }
            _ => {}
        }

        walk_expression(self, tree, id, expression);
    }

    fn visit_match_case(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
    ) {
        let selector = match match_case {
            dir::MatchCase::Expression { selector, .. } => selector,
            dir::MatchCase::Block { selector, .. } => selector,
        };
        if let dir::MatchSelector::Pattern {
            guard: Some(guard), ..
        } = selector
        {
            self.check_condition(*guard, "match guard");
        }

        walk_match_case(self, tree, id, match_case);
    }
}

/// Truthiness certainty for a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Truthiness {
    /// The type is always truthy.
    AlwaysTruthy,
    /// The type is always falsy.
    AlwaysFalsy,
    /// The type can be both or is unknown.
    Unknown,
}

/// Nullish certainty for a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Nullishness {
    /// The type can never be nullish.
    NeverNullish,
    /// The type is always nullish.
    AlwaysNullish,
    /// The type can be nullish or non-nullish.
    MaybeNullish,
}

/// Resolve truthiness certainty for a type.
fn type_truthiness(
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
) -> Truthiness {
    let normalized_type_id = normalize_type(types, type_id);

    let mut visited = HashSet::new();
    type_truthiness_inner(types, strings, normalized_type_id, &mut visited)
}

/// Resolve nullish certainty for a type.
fn type_nullishness(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> Nullishness {
    let normalized_type_id = normalize_type(types, type_id);

    let mut visited = HashSet::new();
    type_nullishness_inner(types, normalized_type_id, &mut visited)
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

/// Resolve truthiness recursively with cycle protection.
fn type_truthiness_inner(
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
    visited: &mut HashSet<dir::LocalTypeId>,
) -> Truthiness {
    if !visited.insert(type_id) {
        return Truthiness::Unknown;
    }

    match types.get_type(type_id) {
        dir::Type::TypeLiteral { value } => match value {
            dir::TypeLiteral::Never => Truthiness::Unknown,
            dir::TypeLiteral::Any | dir::TypeLiteral::Infer | dir::TypeLiteral::Unknown => {
                Truthiness::Unknown
            }
            dir::TypeLiteral::Void => Truthiness::AlwaysFalsy,
            dir::TypeLiteral::Null | dir::TypeLiteral::Undefined => Truthiness::AlwaysFalsy,
            dir::TypeLiteral::Object => Truthiness::AlwaysTruthy,
            dir::TypeLiteral::Primitive(primitive) => match primitive {
                dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol => {
                    Truthiness::AlwaysTruthy
                }
                _ => Truthiness::Unknown,
            },
            dir::TypeLiteral::ScalarLiteral(literal) => match literal {
                dir::ScalarLiteral::Boolean(value) => {
                    if *value {
                        Truthiness::AlwaysTruthy
                    } else {
                        Truthiness::AlwaysFalsy
                    }
                }
                dir::ScalarLiteral::Integer(value) => {
                    if *value == 0 {
                        Truthiness::AlwaysFalsy
                    } else {
                        Truthiness::AlwaysTruthy
                    }
                }
                dir::ScalarLiteral::Bigint(value) => {
                    if *value == 0 {
                        Truthiness::AlwaysFalsy
                    } else {
                        Truthiness::AlwaysTruthy
                    }
                }
                dir::ScalarLiteral::Float(value) => {
                    if *value == 0.0 {
                        Truthiness::AlwaysFalsy
                    } else {
                        Truthiness::AlwaysTruthy
                    }
                }
                dir::ScalarLiteral::String(value) => {
                    if strings.get(*value).is_empty() {
                        Truthiness::AlwaysFalsy
                    } else {
                        Truthiness::AlwaysTruthy
                    }
                }
                dir::ScalarLiteral::Character(_) | dir::ScalarLiteral::RegexString { .. } => {
                    Truthiness::Unknown
                }
            },
            dir::TypeLiteral::Intrinsic(_) => Truthiness::Unknown,
        },
        dir::Type::Value { value } => type_truthiness_inner(types, strings, *value, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            type_truthiness_inner(types, strings, *right, visited)
        }
        dir::Type::Array { .. }
        | dir::Type::ArraySized { .. }
        | dir::Type::Tuple { .. }
        | dir::Type::Object { .. }
        | dir::Type::Function { .. } => Truthiness::AlwaysTruthy,
        dir::Type::Reference { symbol, .. } => {
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return type_truthiness_inner(types, strings, instance_type_id, visited);
            }
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return type_truthiness_inner(types, strings, value_type_id, visited);
            }

            Truthiness::AlwaysTruthy
        }
        dir::Type::Union { elements } => combine_truthiness(
            elements
                .iter()
                .map(|element| type_truthiness_inner(types, strings, *element, visited)),
        ),
        dir::Type::Intersection { elements } => combine_truthiness(
            elements
                .iter()
                .map(|element| type_truthiness_inner(types, strings, *element, visited)),
        ),
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
        | dir::Type::Error => Truthiness::Unknown,
    }
}

/// Resolve nullishness recursively with cycle protection.
fn type_nullishness_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited: &mut HashSet<dir::LocalTypeId>,
) -> Nullishness {
    if !visited.insert(type_id) {
        return Nullishness::MaybeNullish;
    }

    match types.get_type(type_id) {
        dir::Type::TypeLiteral { value } => match value {
            dir::TypeLiteral::Null | dir::TypeLiteral::Undefined => Nullishness::AlwaysNullish,
            dir::TypeLiteral::Never => Nullishness::MaybeNullish,
            dir::TypeLiteral::Any | dir::TypeLiteral::Infer | dir::TypeLiteral::Unknown => {
                Nullishness::MaybeNullish
            }
            dir::TypeLiteral::Void => Nullishness::AlwaysNullish,
            dir::TypeLiteral::Object
            | dir::TypeLiteral::Primitive(_)
            | dir::TypeLiteral::Intrinsic(_)
            | dir::TypeLiteral::ScalarLiteral(_) => Nullishness::NeverNullish,
        },
        dir::Type::Value { value } => type_nullishness_inner(types, *value, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => type_nullishness_inner(types, *right, visited),
        dir::Type::Reference { symbol, .. } => {
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return type_nullishness_inner(types, instance_type_id, visited);
            }
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return type_nullishness_inner(types, value_type_id, visited);
            }

            Nullishness::NeverNullish
        }
        dir::Type::Union { elements } => combine_nullishness(
            elements
                .iter()
                .map(|element| type_nullishness_inner(types, *element, visited)),
        ),
        dir::Type::Intersection { elements } => combine_nullishness(
            elements
                .iter()
                .map(|element| type_nullishness_inner(types, *element, visited)),
        ),
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
        | dir::Type::Error => Nullishness::MaybeNullish,
        dir::Type::Array { .. }
        | dir::Type::ArraySized { .. }
        | dir::Type::Tuple { .. }
        | dir::Type::Object { .. }
        | dir::Type::Function { .. } => Nullishness::NeverNullish,
    }
}

/// Combine truthiness from nested types.
fn combine_truthiness(values: impl Iterator<Item = Truthiness>) -> Truthiness {
    let mut saw_truthy = false;
    let mut saw_falsy = false;

    for value in values {
        match value {
            Truthiness::AlwaysTruthy => saw_truthy = true,
            Truthiness::AlwaysFalsy => saw_falsy = true,
            Truthiness::Unknown => return Truthiness::Unknown,
        }
    }

    match (saw_truthy, saw_falsy) {
        (true, false) => Truthiness::AlwaysTruthy,
        (false, true) => Truthiness::AlwaysFalsy,
        _ => Truthiness::Unknown,
    }
}

/// Combine nullishness from nested types.
fn combine_nullishness(values: impl Iterator<Item = Nullishness>) -> Nullishness {
    let mut saw_nullish = false;
    let mut saw_non_nullish = false;

    for value in values {
        match value {
            Nullishness::AlwaysNullish => saw_nullish = true,
            Nullishness::NeverNullish => saw_non_nullish = true,
            Nullishness::MaybeNullish => return Nullishness::MaybeNullish,
        }
    }

    match (saw_nullish, saw_non_nullish) {
        (true, false) => Nullishness::AlwaysNullish,
        (false, true) => Nullishness::NeverNullish,
        _ => Nullishness::MaybeNullish,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag always truthy object conditions.
    #[test]
    fn test_flags_always_truthy_object_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_always_truthy_object_condition.ts",
            r#"
let value: { x: number } = { x: 1 };
if (value) {
}
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag always falsy null conditions.
    #[test]
    fn test_flags_always_falsy_null_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_always_falsy_null_condition.ts",
            r#"
let value: null = null;
if (value) {
}
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow normal boolean conditions.
    #[test]
    fn test_allows_boolean_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_boolean_condition.ts",
            r#"
let value: boolean = true;
if (value) {
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Allow maybe-nullish union conditions.
    #[test]
    fn test_allows_maybe_nullish_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_maybe_nullish_condition.ts",
            r#"
let value: string | null = "ready";
if (value) {
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Flag nullish coalescing with never-nullish left side.
    #[test]
    fn test_flags_coalesce_left_never_nullish() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_coalesce_left_never_nullish.ts",
            r#"
let value: string = "ready";
let output = value ?? "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag nullish coalescing with always-nullish left side.
    #[test]
    fn test_flags_coalesce_left_always_nullish() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_coalesce_left_always_nullish.ts",
            r#"
let value: null = null;
let output = value ?? "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow nullish coalescing with maybe-nullish left side.
    #[test]
    fn test_allows_coalesce_left_maybe_nullish() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_coalesce_left_maybe_nullish.ts",
            r#"
let value: string | null = null;
let output = value ?? "fallback";
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Flag coalescing with never-nullish array values.
    #[test]
    fn test_flags_coalesce_left_array_never_nullish() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_coalesce_left_array_never_nullish.ts",
            r#"
let value: string[] = [];
let output = value ?? ["fallback"];
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow coalescing when left side is unknown.
    #[test]
    fn test_allows_coalesce_left_unknown() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_coalesce_left_unknown.ts",
            r#"
let value: unknown = null;
let output = value ?? "fallback";
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Flag coalescing when left side is void.
    #[test]
    fn test_flags_coalesce_left_void() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_coalesce_left_void.ts",
            r#"
declare function fetchNothing(): void;
let value = fetchNothing();
let output = value ?? "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag && with always truthy left side.
    #[test]
    fn test_flags_and_left_always_truthy() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_and_left_always_truthy.ts",
            r#"
let value: { ready: boolean } = { ready: true };
let output = value && "ok";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag && with always falsy left side.
    #[test]
    fn test_flags_and_left_always_falsy() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_and_left_always_falsy.ts",
            r#"
let value: null = null;
let output = value && "ok";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag || with always truthy left side.
    #[test]
    fn test_flags_or_left_always_truthy() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_or_left_always_truthy.ts",
            r#"
let value: { ready: boolean } = { ready: true };
let output = value || "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag || with always falsy left side.
    #[test]
    fn test_flags_or_left_always_falsy() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_or_left_always_falsy.ts",
            r#"
let value: null = null;
let output = value || "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow logical operators with normal boolean left side.
    #[test]
    fn test_allows_logical_left_boolean() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_logical_left_boolean.ts",
            r#"
let value: boolean = true;
let a = value && "ok";
let b = value || "fallback";
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }
}
