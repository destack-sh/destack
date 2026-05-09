use std::collections::HashSet;

use destack_ast as ast;
use destack_dir::{self as dir, WellKnownSymbol};
use destack_workspace::{ArrayTypeStyle, LintSeverity};

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{expression_type_map, is_array_type, well_known_symbol_candidates};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

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
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
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
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let preferred_style = ctx.options.style.array_type;
        let array_symbols = resolve_array_symbols(ctx);
        if array_symbols.is_empty() {
            return;
        }
        let mut reported_source_ids = HashSet::new();

        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let Some(source_expression_id) =
                ctx.source_node_id::<ast::Expression>(expression_id.into_any())
            else {
                continue;
            };

            let Some(form) = source_array_form(ctx.ast, source_expression_id) else {
                continue;
            };
            if !expression_is_array_semantic(ctx, expression_id, &array_symbols) {
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

            let span = ctx.ast.get_span(source_expression_id);
            let mut diagnostic = LintReport::new(
                ARRAY_TYPE.id,
                ARRAY_TYPE.code,
                ARRAY_TYPE.category,
                severity,
                mismatch_message(preferred_style),
                span,
            )
            .label(mismatch_label(preferred_style));

            if ctx.include_fixes
                && let Some(fix) = array_type_fix(ctx, source_expression_id, form, preferred_style)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Resolve all concrete `Array` symbols from type and value spaces.
fn resolve_array_symbols(ctx: &LintModuleDirContext<'_>) -> Vec<dir::GlobalSymbolId> {
    let Some(well_known_symbols) = ctx.get_well_known_symbols() else {
        return Vec::new();
    };

    well_known_symbol_candidates(&well_known_symbols, WellKnownSymbol::Array)
}

/// Array type source form.
#[derive(Debug, Clone, Copy)]
enum ArrayTypeForm {
    /// `T[]`.
    Shorthand {
        /// The element type expression.
        element_type_expression_id: ast::LocalNodeId<ast::TypeExpression>,
    },
    /// `Array<T>`.
    Generic {
        /// The first type argument expression.
        type_argument_expression_id: ast::LocalNodeId<ast::TypeExpression>,
    },
}

/// Return the source array form for one AST expression when applicable.
fn source_array_form(
    tree: &ast::Tree,
    source_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ArrayTypeForm> {
    let expression = tree.get(source_expression_id);
    let ast::Expression::Type {
        value: type_expression_id,
    } = expression
    else {
        return None;
    };

    source_array_type_form(tree, *type_expression_id)
}

/// Return the source array form for one AST type expression when applicable.
fn source_array_type_form(
    tree: &ast::Tree,
    type_expression_id: ast::LocalNodeId<ast::TypeExpression>,
) -> Option<ArrayTypeForm> {
    let type_expression = tree.get(type_expression_id);

    // shorthand: T[]
    if let ast::TypeExpression::Array { element } = type_expression {
        return Some(ArrayTypeForm::Shorthand {
            element_type_expression_id: *element,
        });
    }

    // generic path or member: Array<T>
    let generic_arguments = match type_expression {
        ast::TypeExpression::Reference {
            generic_arguments, ..
        }
        | ast::TypeExpression::Member {
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

/// Return one type expression id from one AST generic argument.
fn generic_argument_value_type_expression(
    tree: &ast::Tree,
    argument_id: ast::LocalNodeId<ast::GenericArgument>,
) -> Option<ast::LocalNodeId<ast::TypeExpression>> {
    let argument = tree.get(argument_id);
    match argument {
        ast::GenericArgument::Type { value } => Some(*value),
        ast::GenericArgument::Value { .. } => None,
        ast::GenericArgument::Error => None,
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
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    array_symbols: &[dir::GlobalSymbolId],
) -> bool {
    let expression = ctx.tree.get(expression_id);
    if let dir::Expression::Type { value } = expression
        && let Some(type_id) = ctx
            .types
            .get_declared_or_inferred_type_id(value.into_global_any(ctx.module_id()))
    {
        return array_symbols
            .iter()
            .copied()
            .any(|array_symbol| is_array_type(ctx.types, type_id, Some(array_symbol)));
    }

    expression_type_map(
        ctx.artifacts.as_ref(),
        ctx.profile_id,
        ctx.module_id(),
        ctx.tree,
        ctx.types,
        expression_id,
        |types, type_id| {
            array_symbols
                .iter()
                .copied()
                .any(|array_symbol| is_array_type(types, type_id, Some(array_symbol)))
        },
    )
    .unwrap_or(false)
}

/// Return true when one generic array form is spelled with `Array`.
fn generic_form_targets_array_name(
    ctx: &LintModuleDirContext<'_>,
    source_expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let source_span = ctx.ast.get_span(source_expression_id);
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
    ctx: &LintModuleDirContext<'_>,
    source_expression_id: ast::LocalNodeId<ast::Expression>,
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
            let element_type_text = ctx.get_span_text(ctx.ast.get_span(element_type_expression_id));
            format!("Array<{element_type_text}>")
        }
        (
            ArrayTypeForm::Generic {
                type_argument_expression_id,
            },
            ArrayTypeStyle::Array,
        ) => {
            let type_argument_expression = ctx.ast.get(type_argument_expression_id);
            let type_argument_text =
                ctx.get_span_text(ctx.ast.get_span(type_argument_expression_id));
            if type_argument_needs_parentheses(type_argument_expression) {
                format!("({type_argument_text})[]")
            } else {
                format!("{type_argument_text}[]")
            }
        }
        _ => return None,
    };

    let span = ctx.ast.get_span(source_expression_id);
    let edits = ctx.edit_builder().replace(span, replacement).into_edits();
    Some(LintFix::safe("Rewrite array type form").with_edits(edits))
}

/// Return true when one type argument needs parentheses before appending `[]`.
fn type_argument_needs_parentheses(expression: &ast::TypeExpression) -> bool {
    !matches!(
        expression,
        ast::TypeExpression::Parenthesized { .. }
            | ast::TypeExpression::ScalarLiteral { .. }
            | ast::TypeExpression::Literal { .. }
            | ast::TypeExpression::Intrinsic
            | ast::TypeExpression::Tuple { .. }
            | ast::TypeExpression::Array { .. }
            | ast::TypeExpression::Slice { .. }
            | ast::TypeExpression::Object { .. }
            | ast::TypeExpression::Declaration { .. }
            | ast::TypeExpression::FunctionTypeDeclaration(_)
            | ast::TypeExpression::ConstructorTypeDeclaration(_)
            | ast::TypeExpression::Reference { .. }
            | ast::TypeExpression::Member { .. }
            | ast::TypeExpression::Const
            | ast::TypeExpression::This
            | ast::TypeExpression::Readonly { .. }
            | ast::TypeExpression::KeyOf { .. }
            | ast::TypeExpression::TypeOfValue { .. }
            | ast::TypeExpression::Must { .. }
            | ast::TypeExpression::AsComptime { .. }
            | ast::TypeExpression::Not { .. }
            | ast::TypeExpression::OwnedOf { .. }
            | ast::TypeExpression::BorrowedOf { .. }
            | ast::TypeExpression::PointerOf { .. }
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
