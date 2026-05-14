use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_type_map, is_numeric_property_key_type, is_string_like_property_key_type,
    is_symbol_like_property_key_type,
};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect object literal forms
        for expression_id in ctx.dir.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.dir.get(expression_id);
            let properties = match expression {
                dir::Expression::ObjectExpression { properties, .. } => properties,
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
                LintReport::new(
                    NO_MIXED_KEY_TYPES.id,
                    NO_MIXED_KEY_TYPES.code,
                    NO_MIXED_KEY_TYPES.category,
                    severity,
                    "mixed object key kinds",
                    span,
                )
                .label(
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
    ctx: &LintModuleContext<'_>,
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
    ctx: &LintModuleContext<'_>,
    property_id: dir::LocalNodeId<dir::Property>,
) -> Option<(ObjectKeyKind, destack_source::Span)> {
    let property = ctx.dir.get(property_id);
    let key = match property {
        dir::Property::Field { key, .. } => *key,
        dir::Property::Method { key: Some(key), .. } => *key,
        dir::Property::Method { key: None, .. }
        | dir::Property::Spread { .. }
        | dir::Property::Error => return None,
    };

    let key_kind = match key {
        dir::Key::Name(dir::Name::Identifier(_) | dir::Name::String(_)) => {
            ObjectKeyKind::StringLike
        }
        dir::Key::Name(dir::Name::Number(_)) => ObjectKeyKind::Numeric,
        dir::Key::Private(_) => ObjectKeyKind::SymbolLike,
        dir::Key::Expression(expression_id) => key_expression_kind(ctx, expression_id)?,
    };

    Some((key_kind, ctx.get_span(property_id)))
}

/// Resolve one dynamic key expression into a coarse key kind.
fn key_expression_kind(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ObjectKeyKind> {
    let expression = ctx.dir.get(expression_id);

    // scalar literal keys are statically classifiable
    if let dir::Expression::ScalarLiteral(value) = expression {
        return match value {
            dir::ScalarLiteral::Null => Some(ObjectKeyKind::StringLike),
            dir::ScalarLiteral::String(_)
            | dir::ScalarLiteral::Boolean(_)
            | dir::ScalarLiteral::Character(_)
            | dir::ScalarLiteral::RegexString { .. } => Some(ObjectKeyKind::StringLike),
            dir::ScalarLiteral::Integer(_)
            | dir::ScalarLiteral::Float(_)
            | dir::ScalarLiteral::Bigint(_) => Some(ObjectKeyKind::Numeric),
        };
    }

    // resolve and classify expression types through a single DIR lookup
    expression_type_map(
        ctx.artifacts.as_ref(),
        ctx.profile_id,
        ctx.module_id(),
        ctx.dir.tree(),
        ctx.types,
        expression_id,
        |types, type_id| {
            if is_symbol_like_property_key_type(types, type_id) {
                return Some(ObjectKeyKind::SymbolLike);
            }
            if is_numeric_property_key_type(types, type_id) {
                return Some(ObjectKeyKind::Numeric);
            }
            if is_string_like_property_key_type(types, type_id) {
                return Some(ObjectKeyKind::StringLike);
            }

            None
        },
    )
    .flatten()
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

    /// Flag mixed key kinds for alias-typed computed keys.
    #[test]
    fn test_flags_alias_typed_mixed_computed_keys() {
        let test = TestProgram::for_rule_without_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_flags_alias_typed_mixed_computed_keys.ds",
            r#"
type StringKey = string;
type NumericKey = int32;

const stringKey: StringKey = "name";
const numericKey: NumericKey = 1;

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

    /// Flag mixed key kinds for symbol and numeric keys.
    #[test]
    fn test_flags_mixed_symbol_and_numeric_keys() {
        let test = TestProgram::for_rule_with_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_flags_mixed_symbol_and_numeric_keys.ds",
            r#"
const symbolKey = Symbol("id");
const value = {
    [symbolKey]: 1,
    1: 2,
};
"#,
        );
        test.result(result).assert_lint("no-mixed-key-types");
    }

    /// Allow objects with consistently symbol-like keys.
    #[test]
    fn test_allows_consistent_symbol_keys() {
        let test = TestProgram::for_rule_with_prelude(NoMixedKeyTypes);
        let result = test.lint_dir(
            "no_mixed_key_types/test_allows_consistent_symbol_keys.ds",
            r#"
const first = Symbol("first");
const second = Symbol("second");
const value = {
    [first]: 1,
    [second]: 2,
};
"#,
        );
        test.result(result).assert_no_lint("no-mixed-key-types");
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
