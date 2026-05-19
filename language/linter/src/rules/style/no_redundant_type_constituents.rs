use std::collections::HashMap;

use destack_dir as dir;
use destack_source::LabeledSpan;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    binary_expression_chain_members, binary_expression_is_nested_same_operator,
    expression_is_in_type_position, expression_type_map, normalized_flow_type_id,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow type constituents made redundant by stronger constituents.
    ///
    /// Examples:
    /// - `string | "x"` has a redundant string literal branch.
    /// - `T | never` has a redundant `never`.
    /// - `T & unknown` has a redundant `unknown`.
    #[lint(
        id = "no-redundant-type-constituents",
        code = "LY074",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoRedundantTypeConstituents,
    "Disallow type constituents made redundant by others"
}

impl LintRule for NoRedundantTypeConstituents {
    fn meta(&self) -> &'static LintMeta {
        NoRedundantTypeConstituents::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect top-level type union/intersection chains
        for expression_id in ctx.dir.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.dir.get(expression_id);
            let dir::Expression::Binary { operator, .. } = expression else {
                continue;
            };

            let chain_kind = match operator {
                dir::BinaryOperator::ElementwiseOr => TypeConstituentChainKind::Union,
                dir::BinaryOperator::ElementwiseAnd => TypeConstituentChainKind::Intersection,
                _ => continue,
            };
            if binary_expression_is_nested_same_operator(ctx.dir.tree(), expression_id, *operator) {
                continue;
            }
            if !expression_is_in_type_position(ctx.dir.tree(), expression_id) {
                continue;
            }

            report_redundant_constituents(ctx, meta, expression_id, chain_kind);
        }
    }
}

/// One type constituent chain kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeConstituentChainKind {
    /// Union chain: `A | B`.
    Union,
    /// Intersection chain: `A & B`.
    Intersection,
}

/// One flattened constituent with source and type metadata.
#[derive(Debug, Clone, Copy)]
struct Constituent {
    /// The expression node id.
    expression_id: dir::LocalNodeId<dir::Expression>,
    /// The normalized type id for relation checks.
    normalized_type_id: dir::LocalTypeId,
}

/// Report redundant constituents in one top-level chain.
fn report_redundant_constituents(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
    chain_kind: TypeConstituentChainKind,
) {
    let mut expression_constituents = Vec::new();
    binary_expression_chain_members(
        ctx.dir.tree(),
        expression_id,
        chain_kind.operator(),
        &mut expression_constituents,
    );
    if expression_constituents.len() < 2 {
        return;
    }

    // resolve normalized type ids for each constituent
    let mut constituents = Vec::new();
    for constituent_expression_id in expression_constituents {
        let Some(type_id) = expression_type_map(
            ctx.artifacts.as_ref(),
            ctx.profile_id,
            ctx.module_id(),
            ctx.dir.tree(),
            ctx.types,
            ctx.resolutions,
            constituent_expression_id,
            |_, type_id| type_id,
        ) else {
            return;
        };
        let normalized_type_id = normalized_flow_type_id(ctx.types, type_id);
        constituents.push(Constituent {
            expression_id: constituent_expression_id,
            normalized_type_id,
        });
    }

    // compute redundant indices from chain semantics
    let mut redundant_indices = vec![false; constituents.len()];
    let mut dominant_by_redundant = HashMap::<usize, usize>::new();

    mark_top_bottom_redundancies(
        ctx.types,
        &constituents,
        chain_kind,
        &mut redundant_indices,
        &mut dominant_by_redundant,
    );
    mark_literal_primitive_redundancies(
        ctx.types,
        &constituents,
        chain_kind,
        &mut redundant_indices,
        &mut dominant_by_redundant,
    );
    mark_semantically_equivalent_redundancies(
        &constituents,
        &mut redundant_indices,
        &mut dominant_by_redundant,
    );

    if !redundant_indices.iter().any(|is_redundant| *is_redundant) {
        return;
    }

    // build one replacement expression for a non-overlapping auto-fix
    let replacement = reduced_chain_replacement(ctx, &constituents, chain_kind, &redundant_indices);

    for (index, is_redundant) in redundant_indices.iter().enumerate() {
        if !*is_redundant {
            continue;
        }

        let constituent = constituents[index];
        let severity = ctx.get_effective_severity(meta, constituent.expression_id);
        if !severity.is_enabled() {
            continue;
        }

        let redundant_span = ctx.get_span(constituent.expression_id);
        let mut diagnostic = LintReport::new(
            NO_REDUNDANT_TYPE_CONSTITUENTS.id,
            NO_REDUNDANT_TYPE_CONSTITUENTS.code,
            NO_REDUNDANT_TYPE_CONSTITUENTS.category,
            severity,
            "redundant type constituent",
            redundant_span,
        )
        .label("this constituent does not affect the resulting type");

        if let Some(dominant_index) = dominant_by_redundant.get(&index).copied() {
            let dominant_span = ctx.get_span(constituents[dominant_index].expression_id);
            diagnostic = diagnostic.secondary(LabeledSpan::new(
                dominant_span,
                "this constituent already determines the combined type",
            ));
        }

        // attach one replacement fix once to avoid overlapping edit conflicts
        if index
            == redundant_indices
                .iter()
                .position(|is_marked_redundant| *is_marked_redundant)
                .unwrap_or(index)
            && ctx.compute_fixes
            && let Some(replacement_text) = replacement.as_ref()
        {
            let chain_span = ctx.get_span(expression_id);
            let edits = ctx
                .edit_builder()
                .replace(chain_span, replacement_text)
                .into_edits();
            let fix = LintFix::safe("Remove redundant type constituents").with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// Mark redundancies from top and bottom type constituents.
fn mark_top_bottom_redundancies(
    types: &dir::TypeTable<'_>,
    constituents: &[Constituent],
    chain_kind: TypeConstituentChainKind,
    redundant_indices: &mut [bool],
    dominant_by_redundant: &mut HashMap<usize, usize>,
) {
    let top_indices: Vec<usize> = constituents
        .iter()
        .enumerate()
        .filter_map(|(index, constituent)| {
            top_rank(types, constituent.normalized_type_id, chain_kind).map(|_| index)
        })
        .collect();
    let bottom_indices: Vec<usize> = constituents
        .iter()
        .enumerate()
        .filter_map(|(index, constituent)| {
            bottom_rank(types, constituent.normalized_type_id, chain_kind).map(|_| index)
        })
        .collect();

    // union: top dominates all other constituents
    // intersection: bottom dominates all other constituents
    if chain_kind == TypeConstituentChainKind::Union {
        if let Some(dominant_index) = choose_best_top(types, constituents, &top_indices, chain_kind)
        {
            for (index, is_redundant) in redundant_indices
                .iter_mut()
                .enumerate()
                .take(constituents.len())
            {
                if index == dominant_index {
                    continue;
                }
                *is_redundant = true;
                dominant_by_redundant.entry(index).or_insert(dominant_index);
            }
            return;
        }
    } else if let Some(dominant_index) =
        choose_best_bottom(types, constituents, &bottom_indices, chain_kind)
    {
        for (index, is_redundant) in redundant_indices
            .iter_mut()
            .enumerate()
            .take(constituents.len())
        {
            if index == dominant_index {
                continue;
            }
            *is_redundant = true;
            dominant_by_redundant.entry(index).or_insert(dominant_index);
        }
        return;
    } else if let Some(dominant_index) =
        choose_best_top(types, constituents, &top_indices, chain_kind)
    {
        for (index, is_redundant) in redundant_indices
            .iter_mut()
            .enumerate()
            .take(constituents.len())
        {
            if index == dominant_index {
                continue;
            }
            *is_redundant = true;
            dominant_by_redundant.entry(index).or_insert(dominant_index);
        }
        return;
    }

    // union: `never` is redundant against any non-never constituent
    // intersection: `unknown` is redundant against any non-unknown constituent
    for index in bottom_indices {
        if chain_kind == TypeConstituentChainKind::Union && constituents.len() > 1 {
            redundant_indices[index] = true;
        }
    }

    for (index, constituent) in constituents.iter().enumerate() {
        let is_unknown = matches!(
            types.get_type(constituent.normalized_type_id),
            dir::Type::Unknown
        );
        if chain_kind == TypeConstituentChainKind::Intersection
            && is_unknown
            && constituents.len() > 1
        {
            redundant_indices[index] = true;
        }
    }
}

/// Mark literal and primitive redundancies within one chain.
fn mark_literal_primitive_redundancies(
    types: &dir::TypeTable<'_>,
    constituents: &[Constituent],
    chain_kind: TypeConstituentChainKind,
    redundant_indices: &mut [bool],
    dominant_by_redundant: &mut HashMap<usize, usize>,
) {
    for (left_index, left_constituent) in constituents.iter().enumerate() {
        let left_kind = type_literal_kind(types, left_constituent.normalized_type_id);
        for (right_index, right_constituent) in constituents.iter().enumerate() {
            if left_index == right_index {
                continue;
            }

            let right_kind = type_literal_kind(types, right_constituent.normalized_type_id);
            if is_literal_redundant_against(left_kind, right_kind, chain_kind) {
                redundant_indices[left_index] = true;
                dominant_by_redundant
                    .entry(left_index)
                    .or_insert(right_index);
            }
        }
    }
}

/// Mark later constituents with the same normalized type as earlier ones.
fn mark_semantically_equivalent_redundancies(
    constituents: &[Constituent],
    redundant_indices: &mut [bool],
    dominant_by_redundant: &mut HashMap<usize, usize>,
) {
    // remove later equivalent constituents and preserve source order
    for left_index in 0..constituents.len() {
        let left_type_id = constituents[left_index].normalized_type_id;

        for right_index in (left_index + 1)..constituents.len() {
            if redundant_indices[right_index] {
                continue;
            }

            let right_type_id = constituents[right_index].normalized_type_id;
            if left_type_id != right_type_id {
                continue;
            }

            redundant_indices[right_index] = true;
            dominant_by_redundant
                .entry(right_index)
                .or_insert(left_index);
        }
    }
}

/// Choose the strongest top constituent index.
fn choose_best_top(
    types: &dir::TypeTable<'_>,
    constituents: &[Constituent],
    top_indices: &[usize],
    chain_kind: TypeConstituentChainKind,
) -> Option<usize> {
    top_indices.iter().copied().max_by_key(|index| {
        top_rank(types, constituents[*index].normalized_type_id, chain_kind).unwrap_or(0)
    })
}

/// Choose the strongest bottom constituent index.
fn choose_best_bottom(
    types: &dir::TypeTable<'_>,
    constituents: &[Constituent],
    bottom_indices: &[usize],
    chain_kind: TypeConstituentChainKind,
) -> Option<usize> {
    bottom_indices.iter().copied().max_by_key(|index| {
        bottom_rank(types, constituents[*index].normalized_type_id, chain_kind).unwrap_or(0)
    })
}

/// Return one reduced replacement text by removing redundant constituents.
fn reduced_chain_replacement(
    ctx: &LintModuleContext<'_>,
    constituents: &[Constituent],
    chain_kind: TypeConstituentChainKind,
    redundant_indices: &[bool],
) -> Option<String> {
    let mut kept_parts = Vec::new();
    for (index, constituent) in constituents.iter().enumerate() {
        if redundant_indices[index] {
            continue;
        }

        let text = ctx
            .get_span_text(ctx.get_span(constituent.expression_id))
            .trim()
            .to_string();
        if !text.is_empty() {
            kept_parts.push(text);
        }
    }

    if kept_parts.is_empty() || kept_parts.len() == constituents.len() {
        return None;
    }

    Some(kept_parts.join(chain_kind.separator()))
}

/// Return one top-rank score for a type constituent when it is a top element.
fn top_rank(
    types: &dir::TypeTable<'_>,
    type_id: dir::LocalTypeId,
    chain_kind: TypeConstituentChainKind,
) -> Option<u8> {
    if chain_kind == TypeConstituentChainKind::Union {
        if matches!(types.get_type(type_id), dir::Type::Any) {
            return Some(3);
        }
        if matches!(types.get_type(type_id), dir::Type::Unknown) {
            return Some(2);
        }
    } else if matches!(types.get_type(type_id), dir::Type::Any) {
        return Some(3);
    }

    None
}

/// Return one bottom-rank score for a type constituent when it is a bottom element.
fn bottom_rank(
    types: &dir::TypeTable<'_>,
    type_id: dir::LocalTypeId,
    chain_kind: TypeConstituentChainKind,
) -> Option<u8> {
    let _ = chain_kind;
    if matches!(types.get_type(type_id), dir::Type::Never) {
        return Some(3);
    }

    None
}

/// One comparable literal or primitive kind for redundancy checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeLiteralKind {
    /// Primitive boolean.
    PrimitiveBoolean,
    /// Primitive character.
    PrimitiveCharacter,
    /// Primitive string.
    PrimitiveString,
    /// Primitive integer family.
    PrimitiveInteger,
    /// Primitive float family.
    PrimitiveFloat,
    /// Primitive bigint.
    PrimitiveBigint,
    /// Boolean scalar literal.
    ScalarBoolean,
    /// Character scalar literal.
    ScalarCharacter,
    /// String scalar literal.
    ScalarString,
    /// Integer scalar literal.
    ScalarInteger,
    /// Float scalar literal.
    ScalarFloat,
    /// Bigint scalar literal.
    ScalarBigint,
}

/// Return one comparable literal or primitive kind for a type id.
fn type_literal_kind(
    types: &dir::TypeTable<'_>,
    type_id: dir::LocalTypeId,
) -> Option<TypeLiteralKind> {
    let ty = types.get_type(type_id);
    match ty {
        dir::Type::Primitive(dir::PrimitiveType::Boolean) => {
            Some(TypeLiteralKind::PrimitiveBoolean)
        }
        dir::Type::Primitive(dir::PrimitiveType::Character) => {
            Some(TypeLiteralKind::PrimitiveCharacter)
        }
        dir::Type::Primitive(dir::PrimitiveType::String) => Some(TypeLiteralKind::PrimitiveString),
        dir::Type::Primitive(dir::PrimitiveType::Integer(_)) => {
            Some(TypeLiteralKind::PrimitiveInteger)
        }
        dir::Type::Primitive(dir::PrimitiveType::Float(_)) => Some(TypeLiteralKind::PrimitiveFloat),
        dir::Type::Primitive(dir::PrimitiveType::Bigint) => Some(TypeLiteralKind::PrimitiveBigint),
        dir::Type::Literal(dir::ScalarLiteral::Boolean(_)) => Some(TypeLiteralKind::ScalarBoolean),
        dir::Type::Literal(dir::ScalarLiteral::Character(_)) => {
            Some(TypeLiteralKind::ScalarCharacter)
        }
        dir::Type::Literal(dir::ScalarLiteral::String(_)) => Some(TypeLiteralKind::ScalarString),
        dir::Type::Literal(dir::ScalarLiteral::Integer(_)) => Some(TypeLiteralKind::ScalarInteger),
        dir::Type::Literal(dir::ScalarLiteral::Float(_)) => Some(TypeLiteralKind::ScalarFloat),
        dir::Type::Literal(dir::ScalarLiteral::Bigint(_)) => Some(TypeLiteralKind::ScalarBigint),
        _ => None,
    }
}

/// Return true when one literal kind is redundant against another kind for one chain kind.
fn is_literal_redundant_against(
    left_kind: Option<TypeLiteralKind>,
    right_kind: Option<TypeLiteralKind>,
    chain_kind: TypeConstituentChainKind,
) -> bool {
    let Some(left_kind) = left_kind else {
        return false;
    };
    let Some(right_kind) = right_kind else {
        return false;
    };

    match chain_kind {
        // union keeps the broader primitive and drops covered scalar literals
        TypeConstituentChainKind::Union => matches!(
            (left_kind, right_kind),
            (
                TypeLiteralKind::ScalarBoolean,
                TypeLiteralKind::PrimitiveBoolean
            ) | (
                TypeLiteralKind::ScalarCharacter,
                TypeLiteralKind::PrimitiveCharacter
            ) | (
                TypeLiteralKind::ScalarString,
                TypeLiteralKind::PrimitiveString
            ) | (
                TypeLiteralKind::ScalarInteger,
                TypeLiteralKind::PrimitiveInteger
            ) | (
                TypeLiteralKind::ScalarFloat,
                TypeLiteralKind::PrimitiveFloat
            ) | (
                TypeLiteralKind::ScalarBigint,
                TypeLiteralKind::PrimitiveBigint
            )
        ),
        // intersection keeps the narrower scalar literal and drops broad primitives
        TypeConstituentChainKind::Intersection => matches!(
            (left_kind, right_kind),
            (
                TypeLiteralKind::PrimitiveBoolean,
                TypeLiteralKind::ScalarBoolean
            ) | (
                TypeLiteralKind::PrimitiveCharacter,
                TypeLiteralKind::ScalarCharacter
            ) | (
                TypeLiteralKind::PrimitiveString,
                TypeLiteralKind::ScalarString
            ) | (
                TypeLiteralKind::PrimitiveInteger,
                TypeLiteralKind::ScalarInteger
            ) | (
                TypeLiteralKind::PrimitiveFloat,
                TypeLiteralKind::ScalarFloat
            ) | (
                TypeLiteralKind::PrimitiveBigint,
                TypeLiteralKind::ScalarBigint
            )
        ),
    }
}

impl TypeConstituentChainKind {
    /// Return the binary operator for this chain kind.
    fn operator(self) -> dir::BinaryOperator {
        match self {
            TypeConstituentChainKind::Union => dir::BinaryOperator::ElementwiseOr,
            TypeConstituentChainKind::Intersection => dir::BinaryOperator::ElementwiseAnd,
        }
    }

    /// Return the source separator for this chain kind.
    fn separator(self) -> &'static str {
        match self {
            TypeConstituentChainKind::Union => " | ",
            TypeConstituentChainKind::Intersection => " & ",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag `never` as a redundant union constituent.
    #[test]
    fn test_flags_never_in_union() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let result = test.lint_dir(
            "no_redundant_type_constituents/test_flags_never_in_union.ds",
            r#"
type Value = string | never;
"#,
        );
        test.result(result)
            .assert_lint("no-redundant-type-constituents");
    }

    /// Flag other union constituents when `unknown` is present.
    #[test]
    fn test_flags_union_redundant_with_unknown() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let result = test.lint_dir(
            "no_redundant_type_constituents/test_flags_union_redundant_with_unknown.ds",
            r#"
type Value = unknown | string | int32;
"#,
        );
        test.result(result)
            .assert_lint_count("no-redundant-type-constituents", 2)
            .assert_has_fix("no-redundant-type-constituents");
    }

    /// Flag redundant union string literal when `string` is also present.
    #[test]
    fn test_flags_union_literal_redundant_to_primitive() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let result = test.lint_dir(
            "no_redundant_type_constituents/test_flags_union_literal_redundant_to_primitive.ds",
            r#"
type Value = string | "ok" | boolean;
"#,
        );
        test.result(result)
            .assert_lint("no-redundant-type-constituents")
            .assert_safe_fixed(
                r#"
type Value = string | boolean;
"#,
            );
    }

    /// Flag redundant intersection unknown constituent.
    #[test]
    fn test_flags_intersection_unknown_redundant() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let result = test.lint_dir(
            "no_redundant_type_constituents/test_flags_intersection_unknown_redundant.ds",
            r#"
type Value = unknown & { id: string };
"#,
        );
        test.result(result)
            .assert_lint("no-redundant-type-constituents")
            .assert_safe_fixed(
                r#"
type Value = { id: string };
"#,
            );
    }

    /// Flag all other intersection constituents when `never` is present.
    #[test]
    fn test_flags_intersection_dominated_by_never() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let result = test.lint_dir(
            "no_redundant_type_constituents/test_flags_intersection_dominated_by_never.ds",
            r#"
type Value = string & never & { id: string };
"#,
        );
        test.result(result)
            .assert_lint_count("no-redundant-type-constituents", 2)
            .assert_safe_fixed(
                r#"
type Value = never;
"#,
            );
    }

    /// Flag redundant primitive in intersection with a matching literal.
    #[test]
    fn test_flags_intersection_primitive_redundant_to_literal() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let result = test.lint_dir(
            "no_redundant_type_constituents/test_flags_intersection_primitive_redundant_to_literal.ds",
            r#"
type Value = string & "ok";
"#,
        );
        test.result(result)
            .assert_lint("no-redundant-type-constituents")
            .assert_safe_fixed(
                r#"
type Value = "ok";
"#,
            );
    }

    /// Allow non-redundant type chains.
    #[test]
    fn test_allows_non_redundant_type_chain() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let result = test.lint_dir(
            "no_redundant_type_constituents/test_allows_non_redundant_type_chain.ds",
            r#"
type Value = string | int32 | boolean;
"#,
        );
        test.result(result)
            .assert_no_lint("no-redundant-type-constituents");
    }

    /// Flag equivalent union constituents that reference the same imported type symbol.
    #[test]
    fn test_flags_semantically_equivalent_union_import_alias_constituent() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let diagnostics = test.lint_module_dir_with_modules(
            crate::test_modules! {
                "no_redundant_type_constituents/shared.ds" => r#"
export type SharedText = string;
"#,
                "no_redundant_type_constituents/main.ds" => r#"
import { SharedText as Left, SharedText as Right } from "./shared.ds";

type Value = Left | Right;
"#,
            },
            "no_redundant_type_constituents/main.ds",
        );
        test.result(diagnostics)
            .assert_lint("no-redundant-type-constituents")
            .assert_safe_fixed(
                r#"
import { SharedText as Left, SharedText as Right } from "./shared.ds";

type Value = Left;
"#,
            );
    }

    /// Flag equivalent intersection constituents that reference the same imported type symbol.
    #[test]
    fn test_flags_semantically_equivalent_intersection_import_alias_constituent() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let diagnostics = test.lint_module_dir_with_modules(
            crate::test_modules! {
                "no_redundant_type_constituents/shared_intersection.ds" => r#"
export type SharedText = string;
"#,
                "no_redundant_type_constituents/main_intersection.ds" => r#"
import { SharedText as Left, SharedText as Right } from "./shared_intersection.ds";

type Value = Left & Right;
"#,
            },
            "no_redundant_type_constituents/main_intersection.ds",
        );
        test.result(diagnostics)
            .assert_lint("no-redundant-type-constituents")
            .assert_safe_fixed(
                r#"
import { SharedText as Left, SharedText as Right } from "./shared_intersection.ds";

type Value = Left;
"#,
            );
    }

    /// Allow union constituents when imported symbols point to different declarations.
    #[test]
    fn test_allows_non_equivalent_union_import_symbols() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let diagnostics = test.lint_module_dir_with_modules(
            crate::test_modules! {
                "no_redundant_type_constituents/shared_non_equivalent.ds" => r#"
export type LeftText = string;
export type RightText = number;
"#,
                "no_redundant_type_constituents/main_non_equivalent.ds" => r#"
import { LeftText, RightText } from "./shared_non_equivalent.ds";

type Value = LeftText | RightText;
"#,
            },
            "no_redundant_type_constituents/main_non_equivalent.ds",
        );
        test.result(diagnostics)
            .assert_no_lint("no-redundant-type-constituents");
    }

    /// Ignore value-level elementwise expressions.
    #[test]
    fn test_ignores_value_level_elementwise_expression() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantTypeConstituents);
        let result = test.lint_dir(
            "no_redundant_type_constituents/test_ignores_value_level_elementwise_expression.ds",
            r#"
let value = a | b;
"#,
        );
        test.result(result)
            .assert_no_lint("no-redundant-type-constituents");
    }
}
