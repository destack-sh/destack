use std::collections::HashSet;

use destack_dir::{self as dir, LanguageItem};
use destack_repository::{ArrayTypeStyle, LintSeverity};

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{expression_type_map, is_array_type};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Enforce one consistent array type form.
    ///
    /// Configure the preferred form via `array_type` in linter options.
    /// - `ArrayTypeStyle::Array` prefers `T[]`
    /// - `ArrayTypeStyle::Generic` prefers `Array<T>`
    #[lint(
        id = "array-type",
        code = "LY070",
        category = Style,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::Array)],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub ArrayType,
    "Enforce one consistent array type form"
}

impl LintRule for ArrayType {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        ArrayType::meta()
    }

    /// Check module DIR nodes for inconsistent array type forms.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let preferred_style = ctx.options().style.array_type;
        let Some(array_symbol) = ctx.get_language_item(LanguageItem::Array) else {
            return;
        };
        let mut reported_source_ids = HashSet::new();

        for expression_id in ctx.dir.iter_node_ids_of_type::<dir::Expression>() {
            let Some(source_expression_id) =
                ctx.source_node_id::<dir::Expression>(expression_id.into_any())
            else {
                continue;
            };

            let Some(form) = source_array_form(ctx.dir.tree(), source_expression_id) else {
                continue;
            };
            if !expression_is_array_semantic(ctx, expression_id, array_symbol) {
                continue;
            }

            // only enforce generic form when the source expression targets Array directly
            if matches!(form, ArrayTypeForm::Generic { .. })
                && !generic_form_targets_array_name(ctx, source_expression_id)
            {
                continue;
            }

            if form_matches_preference(form, preferred_style) {
                continue;
            }
            if !reported_source_ids.insert(source_expression_id.id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.dir.get_span(source_expression_id);
            let mut diagnostic = LintReport::new(
                ARRAY_TYPE.id,
                ARRAY_TYPE.code,
                ARRAY_TYPE.category,
                severity,
                mismatch_message(preferred_style),
                span,
            )
            .label(mismatch_label(preferred_style));

            if ctx.compute_fixes
                && let Some(fix) = array_type_fix(ctx, source_expression_id, form, preferred_style)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Array type source form.
#[derive(Debug, Clone, Copy)]
enum ArrayTypeForm {
    /// `T[]`.
    Shorthand {
        /// The element type expression.
        element_type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
    },
    /// `Array<T>`.
    Generic {
        /// The first type argument expression.
        type_argument_expression_id: dir::LocalNodeId<dir::TypeExpression>,
    },
}

/// Return the source array form for one source expression when applicable.
fn source_array_form(
    tree: &dir::Tree,
    source_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ArrayTypeForm> {
    let expression = tree.get(source_expression_id);
    let dir::Expression::Type {
        value: type_expression_id,
    } = expression
    else {
        return None;
    };

    source_array_type_form(tree, *type_expression_id)
}

/// Return the source array form for one source type expression when applicable.
fn source_array_type_form(
    tree: &dir::Tree,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> Option<ArrayTypeForm> {
    let type_expression = tree.get(type_expression_id);

    // shorthand: T[]
    if let dir::TypeExpression::Array { element } = type_expression {
        return Some(ArrayTypeForm::Shorthand {
            element_type_expression_id: *element,
        });
    }

    // generic path or member: Array<T>
    let generic_arguments = match type_expression {
        dir::TypeExpression::Reference {
            generic_arguments, ..
        }
        | dir::TypeExpression::Member {
            generic_arguments, ..
        } => generic_arguments,
        _ => return None,
    };
    if generic_arguments.len() != 1 {
        return None;
    }

    let type_argument_expression_id =
        generic_argument_value_type_expression(tree, generic_arguments[0])?;
    Some(ArrayTypeForm::Generic {
        type_argument_expression_id,
    })
}

/// Return one type expression id from one source generic argument.
fn generic_argument_value_type_expression(
    tree: &dir::Tree,
    argument_id: dir::LocalNodeId<dir::GenericArgument>,
) -> Option<dir::LocalNodeId<dir::TypeExpression>> {
    let argument = tree.get(argument_id);
    match argument {
        dir::GenericArgument::Type { value }
        | dir::GenericArgument::AssociatedType { value, .. } => Some(*value),
        dir::GenericArgument::SpreadType { .. }
        | dir::GenericArgument::Value { .. }
        | dir::GenericArgument::SpreadValue { .. }
        | dir::GenericArgument::AssociatedConst { .. } => None,
        dir::GenericArgument::Error => None,
    }
}

/// Return true when the array form matches the configured preference.
fn form_matches_preference(form: ArrayTypeForm, preferred_style: ArrayTypeStyle) -> bool {
    matches!(
        (form, preferred_style),
        (ArrayTypeForm::Shorthand { .. }, ArrayTypeStyle::Array)
            | (ArrayTypeForm::Generic { .. }, ArrayTypeStyle::Generic)
    )
}

/// Return true when one DIR expression resolves to an array type.
fn expression_is_array_semantic(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    array_symbol: dir::GlobalSymbolId,
) -> bool {
    let expression = ctx.dir.get(expression_id);
    if let dir::Expression::Type { value } = expression
        && let Some(type_id) = ctx
            .types
            .get_node_type_id(value.into_global_any(ctx.module_id()))
    {
        return is_array_type(ctx, type_id, Some(array_symbol));
    }

    expression_type_map(ctx, expression_id, |ctx, type_id| {
        is_array_type(ctx, type_id, Some(array_symbol))
    })
    .unwrap_or(false)
}

/// Return true when one generic array form is spelled with `Array`.
fn generic_form_targets_array_name(
    ctx: &LintModuleContext<'_>,
    source_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let source_span = ctx.dir.get_span(source_expression_id);
    let source_text = ctx.get_span_text(source_span);
    let compact_text = source_text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();

    compact_text.starts_with("Array<") || compact_text.contains(".Array<")
}

/// Return the mismatch message for one preferred style.
fn mismatch_message(preferred_style: ArrayTypeStyle) -> &'static str {
    match preferred_style {
        ArrayTypeStyle::Array => "prefer `T[]` over `Array<T>`",
        ArrayTypeStyle::Generic => "prefer `Array<T>` over `T[]`",
    }
}

/// Return the mismatch label for one preferred style.
fn mismatch_label(preferred_style: ArrayTypeStyle) -> &'static str {
    match preferred_style {
        ArrayTypeStyle::Array => "use shorthand array form for this type",
        ArrayTypeStyle::Generic => "use generic array form for this type",
    }
}

/// Build a safe array-style fix.
fn array_type_fix(
    ctx: &LintModuleContext<'_>,
    source_expression_id: dir::LocalNodeId<dir::Expression>,
    form: ArrayTypeForm,
    preferred_style: ArrayTypeStyle,
) -> Option<LintFix> {
    let replacement = match (form, preferred_style) {
        (
            ArrayTypeForm::Shorthand {
                element_type_expression_id,
            },
            ArrayTypeStyle::Generic,
        ) => {
            let element_type_text = ctx.get_span_text(ctx.dir.get_span(element_type_expression_id));
            format!("Array<{element_type_text}>")
        }
        (
            ArrayTypeForm::Generic {
                type_argument_expression_id,
            },
            ArrayTypeStyle::Array,
        ) => {
            let type_argument_expression = ctx.dir.get(type_argument_expression_id);
            let type_argument_text =
                ctx.get_span_text(ctx.dir.get_span(type_argument_expression_id));
            if type_argument_needs_parentheses(type_argument_expression) {
                format!("({type_argument_text})[]")
            } else {
                format!("{type_argument_text}[]")
            }
        }
        _ => return None,
    };

    let span = ctx.dir.get_span(source_expression_id);
    let edits = ctx.edit_builder().replace(span, replacement).into_edits();
    Some(LintFix::safe("Rewrite array type form").with_edits(edits))
}

/// Return true when one type argument needs parentheses before appending `[]`.
fn type_argument_needs_parentheses(expression: &dir::TypeExpression) -> bool {
    !matches!(
        expression,
        dir::TypeExpression::Parenthesized { .. }
            | dir::TypeExpression::ScalarLiteral { value: _ }
            | dir::TypeExpression::Literal { .. }
            | dir::TypeExpression::Intrinsic
            | dir::TypeExpression::Tuple { .. }
            | dir::TypeExpression::Array { .. }
            | dir::TypeExpression::Slice { .. }
            | dir::TypeExpression::Object { .. }
            | dir::TypeExpression::Function(_)
            | dir::TypeExpression::Constructor(_)
            | dir::TypeExpression::Reference { .. }
            | dir::TypeExpression::Member { .. }
            | dir::TypeExpression::Const
            | dir::TypeExpression::This
            | dir::TypeExpression::Readonly { .. }
            | dir::TypeExpression::Local { .. }
            | dir::TypeExpression::Shared { .. }
            | dir::TypeExpression::KeyOf { .. }
            | dir::TypeExpression::TypeOfValue { .. }
            | dir::TypeExpression::Must { .. }
            | dir::TypeExpression::Not { .. }
            | dir::TypeExpression::OwnedOf { .. }
            | dir::TypeExpression::BorrowedOf { .. }
            | dir::TypeExpression::PointerOf { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Prefer shorthand `T[]` by default.
    #[test]
    fn test_flags_generic_array_type_by_default() {
        let test = TestProgram::for_rule_with_prelude(ArrayType);
        let result = test.lint_dir(
            "array_type/test_flags_generic_array_type_by_default.ds",
            r#"
type Values = Array<number>;
"#,
        );
        test.result(result)
            .assert_lint("array-type")
            .assert_has_fix("array-type")
            .assert_safe_fixed(
                r#"
type Values = number[];
"#,
            );
    }

    /// Allow shorthand `T[]` by default.
    #[test]
    fn test_allows_shorthand_array_type_by_default() {
        let test = TestProgram::for_rule_with_prelude(ArrayType);
        let result = test.lint_dir(
            "array_type/test_allows_shorthand_array_type_by_default.ds",
            r#"
type Values = number[];
"#,
        );
        test.result(result).assert_no_lint("array-type");
    }

    /// Prefer generic `Array<T>` when configured.
    #[test]
    fn test_flags_shorthand_array_type_when_generic_preferred() {
        let test = TestProgram::for_rule_with_prelude(ArrayType).with_options(|options| {
            options.style.array_type = ArrayTypeStyle::Generic;
        });
        let result = test.lint_dir(
            "array_type/test_flags_shorthand_array_type_when_generic_preferred.ds",
            r#"
type Values = number[];
"#,
        );
        test.result(result)
            .assert_lint("array-type")
            .assert_has_fix("array-type")
            .assert_safe_fixed(
                r#"
type Values = Array<number>;
"#,
            );
    }

    /// Allow generic `Array<T>` when configured.
    #[test]
    fn test_allows_generic_array_type_when_generic_preferred() {
        let test = TestProgram::for_rule_with_prelude(ArrayType).with_options(|options| {
            options.style.array_type = ArrayTypeStyle::Generic;
        });
        let result = test.lint_dir(
            "array_type/test_allows_generic_array_type_when_generic_preferred.ds",
            r#"
type Values = Array<number>;
"#,
        );
        test.result(result).assert_no_lint("array-type");
    }

    /// Preserve precedence when rewriting generic union element arrays.
    #[test]
    fn test_fix_wraps_union_type_when_rewriting_to_shorthand() {
        let test = TestProgram::for_rule_with_prelude(ArrayType);
        let result = test.lint_dir(
            "array_type/test_fix_wraps_union_type_when_rewriting_to_shorthand.ds",
            r#"
type Values = Array<string | number>;
"#,
        );
        test.result(result)
            .assert_lint("array-type")
            .assert_safe_fixed(
                r#"
type Values = (string | number)[];
"#,
            );
    }

    /// Ignore non-array generic type references.
    #[test]
    fn test_ignores_non_array_generic_types() {
        let test = TestProgram::for_rule_with_prelude(ArrayType);
        let result = test.lint_dir(
            "array_type/test_ignores_non_array_generic_types.ds",
            r#"
type Box<T> = { value: T };
type Values = Box<number>;
"#,
        );
        test.result(result).assert_no_lint("array-type");
    }

    /// Allow aliases that preserve their own generic type style.
    #[test]
    fn test_allows_generic_alias_that_resolves_to_array() {
        let test = TestProgram::for_rule_with_prelude(ArrayType);
        let result = test.lint_dir(
            "array_type/test_allows_generic_alias_that_resolves_to_array.ds",
            r#"
type List<T> = T[];
type Values = List<number>;
"#,
        );
        test.result(result).assert_no_lint("array-type");
    }
}
