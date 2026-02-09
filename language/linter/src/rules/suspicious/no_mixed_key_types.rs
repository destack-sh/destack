use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_declared_or_inferred_type_id;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow object literals that mix incompatible key kinds.
    ///
    /// Mixing string, numeric, and symbol-like keys in one literal makes object
    /// semantics harder to reason about and can hide subtle runtime behavior.
    #[lint(
        id = "no-mixed-key-types",
        code = "LU023",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Experimental,
        declarations = Exclude
    )]
    pub NoMixedKeyTypes,
    "Disallow mixed key kinds in object literals"
}

impl LintRule for NoMixedKeyTypes {
    fn meta(&self) -> &'static LintMeta {
        NoMixedKeyTypes::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect object literal forms
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let properties = match expression {
                dir::Expression::ObjectExpression { properties }
                | dir::Expression::TaggedObjectExpression { properties, .. } => properties,
                _ => continue,
            };

            let key_summary = collect_key_kind_summary(ctx, properties);
            if key_summary.distinct_key_kind_count() < 2 {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = key_summary
                .first_mixed_key_span
                .unwrap_or_else(|| ctx.get_span(expression_id));
            ctx.report(
                LintDiagnostic::new(
                    NO_MIXED_KEY_TYPES.id,
                    NO_MIXED_KEY_TYPES.code,
                    NO_MIXED_KEY_TYPES.category,
                    severity,
                    "mixed object key kinds",
                    ctx.module.file_id,
                    span,
                )
                .with_label(
                    "keep object literal keys consistently string-like, numeric, or symbol-like",
                ),
            );
        }
    }
}

/// One key-kind summary for an object literal.
#[derive(Debug, Default)]
struct KeyKindSummary {
    /// Whether string-like keys were seen.
    has_string_like: bool,
    /// Whether numeric keys were seen.
    has_numeric: bool,
    /// Whether symbol-like keys were seen.
    has_symbol_like: bool,
    /// The first span that contributed to a mixed-key state.
    first_mixed_key_span: Option<destack_source::Span>,
}

impl KeyKindSummary {
    /// Track one key kind.
    fn observe(&mut self, key_kind: ObjectKeyKind, key_span: destack_source::Span) {
        match key_kind {
            ObjectKeyKind::StringLike => self.has_string_like = true,
            ObjectKeyKind::Numeric => self.has_numeric = true,
            ObjectKeyKind::SymbolLike => self.has_symbol_like = true,
        }

        if self.first_mixed_key_span.is_none() && self.distinct_key_kind_count() >= 2 {
            self.first_mixed_key_span = Some(key_span);
        }
    }

    /// Count distinct key kinds observed so far.
    fn distinct_key_kind_count(&self) -> usize {
        usize::from(self.has_string_like)
            + usize::from(self.has_numeric)
            + usize::from(self.has_symbol_like)
    }
}

/// One coarse key kind category.
#[derive(Debug, Clone, Copy)]
enum ObjectKeyKind {
    /// Identifier or string-like keys.
    StringLike,
    /// Number-like keys.
    Numeric,
    /// Symbol-like keys.
    SymbolLike,
}

/// Collect key-kind information for object properties.
fn collect_key_kind_summary(
    ctx: &LintModuleDirContext<'_>,
    properties: &[dir::LocalNodeId<dir::Property>],
) -> KeyKindSummary {
    let mut summary = KeyKindSummary::default();

    // collect known property key kinds
    for property_id in properties {
        let Some((key_kind, key_span)) = property_key_kind(ctx, *property_id) else {
            continue;
        };
        summary.observe(key_kind, key_span);
    }

    summary
}

/// Resolve one property key kind when it is statically classifiable.
fn property_key_kind(
    ctx: &LintModuleDirContext<'_>,
    property_id: dir::LocalNodeId<dir::Property>,
) -> Option<(ObjectKeyKind, destack_source::Span)> {
    let property = ctx.tree.get(property_id);
    let dynamic_key = match property {
        dir::Property::Field { key, .. } | dir::Property::Method { key, .. } => *key,
        dir::Property::Spread { .. } => None,
    }?;

    let key_kind = match dynamic_key {
        dir::DynamicKey::Name(_) => ObjectKeyKind::StringLike,
        dir::DynamicKey::Number(_) => ObjectKeyKind::Numeric,
        dir::DynamicKey::Private(_) => ObjectKeyKind::SymbolLike,
        dir::DynamicKey::Expression(expression_id) => key_expression_kind(ctx, expression_id)?,
        dir::DynamicKey::NamedExpression { key, .. } => key_expression_kind(ctx, key)?,
    };

    Some((key_kind, ctx.get_span(property_id)))
}

/// Resolve one dynamic key expression into a coarse key kind.
fn key_expression_kind(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ObjectKeyKind> {
    let expression = ctx.tree.get(expression_id);

    // scalar literal keys are statically classifiable
    if let dir::Expression::ScalarLiteral { value } = expression {
        return match value {
            dir::ScalarLiteral::String(_)
            | dir::ScalarLiteral::Boolean(_)
            | dir::ScalarLiteral::Character(_)
            | dir::ScalarLiteral::RegexString { .. } => Some(ObjectKeyKind::StringLike),
            dir::ScalarLiteral::Integer(_)
            | dir::ScalarLiteral::Float(_)
            | dir::ScalarLiteral::Bigint(_) => Some(ObjectKeyKind::Numeric),
        };
    }

    // symbol-like primitive key types
    let type_id = expression_declared_or_inferred_type_id(
        ctx.module_id(),
        ctx.tree,
        ctx.types,
        expression_id,
    )
    .or_else(|| ctx.expression_type_id(expression_id))?;
    if expression_type_is_symbol_like(ctx.types, type_id) {
        return Some(ObjectKeyKind::SymbolLike);
    }
    if expression_type_is_numeric(ctx.types, type_id) {
        return Some(ObjectKeyKind::Numeric);
    }
    if expression_type_is_string_like(ctx.types, type_id) {
        return Some(ObjectKeyKind::StringLike);
    }

    None
}

/// Return true when the type is string-like.
fn expression_type_is_string_like(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    let type_id = normalized_type_id(types, type_id);
    match types.get_type(type_id) {
        dir::Type::TypeLiteral { value } => matches!(
            value,
            dir::TypeLiteral::Primitive(dir::PrimitiveType::String)
                | dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean)
                | dir::TypeLiteral::Primitive(dir::PrimitiveType::Character)
                | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::String(_))
                | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Boolean(_))
                | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Character(_))
                | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::RegexString { .. })
                | dir::TypeLiteral::Null
                | dir::TypeLiteral::Undefined
        ),
        dir::Type::Value { value } => expression_type_is_string_like(types, *value),
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| expression_type_is_string_like(types, *element)),
        _ => false,
    }
}

/// Return true when the type is numeric.
fn expression_type_is_numeric(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    let type_id = normalized_type_id(types, type_id);
    match types.get_type(type_id) {
        dir::Type::TypeLiteral { value } => matches!(
            value,
            dir::TypeLiteral::Primitive(dir::PrimitiveType::Number)
                | dir::TypeLiteral::Primitive(dir::PrimitiveType::Bigint)
                | dir::TypeLiteral::Primitive(dir::PrimitiveType::Int(_))
                | dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(_))
                | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Integer(_))
                | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Float(_))
                | dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Bigint(_))
        ),
        dir::Type::Value { value } => expression_type_is_numeric(types, *value),
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| expression_type_is_numeric(types, *element)),
        _ => false,
    }
}

/// Return true when the type is symbol-like.
fn expression_type_is_symbol_like(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    let type_id = normalized_type_id(types, type_id);
    match types.get_type(type_id) {
        dir::Type::TypeLiteral { value } => matches!(
            value,
            dir::TypeLiteral::Primitive(dir::PrimitiveType::Symbol)
                | dir::TypeLiteral::Primitive(dir::PrimitiveType::UniqueSymbol)
        ),
        dir::Type::Value { value } => expression_type_is_symbol_like(types, *value),
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => elements
            .iter()
            .all(|element| expression_type_is_symbol_like(types, *element)),
        _ => false,
    }
}

/// Normalize one type id for stable key-kind checks.
fn normalized_type_id(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> dir::LocalTypeId {
    types
        .normalized_type(dir::NormalizationMode::Flow, 0, type_id)
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag mixed string and numeric keys in one object literal.
    #[test]
    fn test_flags_mixed_string_and_numeric_keys() {
        let test = TestProgram::for_rule_without_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_flags_mixed_string_and_numeric_keys.ds",
            r#"
const value = {
    name: "ok",
    1: "one",
};
"#,
        );
        test.result(result).assert_lint("no-mixed-key-types");
    }

    /// Flag mixed key kinds in tagged object literals.
    #[test]
    fn test_flags_mixed_key_types_in_tagged_object() {
        let test = TestProgram::for_rule_without_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_flags_mixed_key_types_in_tagged_object.ds",
            r#"
type Item = {
    name: string;
    "1": string;
};

const value = Item {
    name: "ok",
    1: "one",
};
"#,
        );
        test.result(result).assert_lint("no-mixed-key-types");
    }

    /// Allow objects with only string-like keys.
    #[test]
    fn test_allows_consistent_string_keys() {
        let test = TestProgram::for_rule_without_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_allows_consistent_string_keys.ds",
            r#"
const value = {
    name: "ok",
    title: "hello",
};
"#,
        );
        test.result(result).assert_no_lint("no-mixed-key-types");
    }

    /// Allow objects with only numeric keys.
    #[test]
    fn test_allows_consistent_numeric_keys() {
        let test = TestProgram::for_rule_without_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_allows_consistent_numeric_keys.ds",
            r#"
const value = {
    1: "one",
    2: "two",
};
"#,
        );
        test.result(result).assert_no_lint("no-mixed-key-types");
    }

    /// Flag mixed key kinds for computed string and numeric keys.
    #[test]
    fn test_flags_mixed_computed_string_and_numeric_keys() {
        let test = TestProgram::for_rule_without_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_flags_mixed_computed_string_and_numeric_keys.ds",
            r#"
const stringKey = "name";
const numericKey = 1;

const value = {
    [stringKey]: "ok",
    [numericKey]: "one",
};
"#,
        );
        test.result(result).assert_lint("no-mixed-key-types");
    }

    /// Flag mixed key kinds for symbol and string keys.
    #[test]
    fn test_flags_mixed_symbol_and_string_keys() {
        let test = TestProgram::for_rule_with_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_flags_mixed_symbol_and_string_keys.ds",
            r#"
const symbolKey = Symbol("id");
const value = {
    [symbolKey]: 1,
    name: 2,
};
"#,
        );
        test.result(result).assert_lint("no-mixed-key-types");
    }

    /// Allow ambiguous computed keys that are not statically classifiable.
    #[test]
    fn test_allows_ambiguous_computed_keys() {
        let test = TestProgram::for_rule_without_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_allows_ambiguous_computed_keys.ds",
            r#"
function key(flag: boolean): string | int32 {
    if (flag) {
        return "name";
    }

    return 1;
}

const value = {
    [key(true)]: "ok",
    title: "hello",
};
"#,
        );
        test.result(result).assert_no_lint("no-mixed-key-types");
    }

    /// Treat boolean computed keys as string-like.
    #[test]
    fn test_allows_boolean_and_string_like_keys() {
        let test = TestProgram::for_rule_without_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_allows_boolean_and_string_like_keys.ds",
            r#"
const value = {
    [true]: "yes",
    name: "ok",
};
"#,
        );
        test.result(result).assert_no_lint("no-mixed-key-types");
    }
}
