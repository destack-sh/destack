use std::collections::HashSet;

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NormalizationMode, PrimitiveType, ScalarLiteral,
    Type, TypeLiteral, TypeTable,
};

use super::{CanonicalSymbolMode, RelationMode};
use crate::analyze::common::{TypeContext, TypeView};
use crate::{AnalyzeResult, Assignability, Compiler, StaticMemberSymbolKind};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return true when one symbol is valid as a symbolic static value reference.
    pub(crate) fn symbol_is_symbolic_static_value_reference(
        &self,
        ctx: TypeView<'_>,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<bool> {
        if self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol) {
            return Ok(true);
        }

        let symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), symbol)
            .unwrap_or(symbol);

        let kind =
            self.query_static_member_symbol_kind_for_symbol(ctx.tree_symbol_view(), symbol)?;
        Ok(matches!(
            kind,
            Some(
                StaticMemberSymbolKind::AssociatedComptimeConst | StaticMemberSymbolKind::EnumField
            )
        ))
    }

    /// Check whether a type requires evaluative normalization.
    pub(crate) fn type_requires_evaluative_normalization(
        &self,
        ctx: TypeView<'_>,
        type_id: LocalTypeId,
    ) -> bool {
        // check for free static parameter references
        let mut static_visited = HashSet::new();
        let bound = HashSet::new();
        if self.type_contains_free_static_parameters(ctx, type_id, &bound, &mut static_visited) {
            return true;
        }

        // check for inference variables
        let mut infer_visited = HashSet::new();
        if self.type_contains_infer_vars(type_id, ctx.types, &mut infer_visited) {
            return true;
        }

        // check for conditional infer bindings
        let mut binding_visited = HashSet::new();
        if self.type_contains_infer(type_id, ctx.types, &mut binding_visited) {
            return true;
        }

        false
    }

    /// Check whether a type requires convergence before static evaluation.
    pub(crate) fn type_requires_static_evaluation_convergence(
        &self,
        ctx: TypeView<'_>,
        type_id: LocalTypeId,
    ) -> bool {
        // evaluation dependent types are not stable yet
        if self.type_requires_evaluative_normalization(ctx, type_id) {
            return true;
        }

        // unevaluated static arguments are not stable yet
        let mut static_argument_visited = HashSet::new();
        if self.type_has_unevaluated_static_arguments(
            type_id,
            ctx.types,
            &mut static_argument_visited,
        ) {
            return true;
        }

        // associated type references require receiver projection substitutions
        let mut associated_reference_visited = HashSet::new();
        if self.type_contains_associated_type_reference(
            ctx.tree_symbol_view(),
            type_id,
            ctx.types,
            &mut associated_reference_visited,
        ) {
            return true;
        }

        false
    }

    /// Check whether a type is converged for static evaluation.
    pub(crate) fn type_is_converged_for_static_evaluation(
        &self,
        ctx: TypeView<'_>,
        type_id: LocalTypeId,
    ) -> bool {
        !self.type_requires_static_evaluation_convergence(ctx, type_id)
    }

    /// Check whether one normalized type stays as deferred `keyof` projection.
    pub(crate) fn type_is_deferred_keyof_projection(
        &self,
        ctx: TypeView<'_>,
        type_id: LocalTypeId,
    ) -> bool {
        match ctx.types.get_type(type_id) {
            Type::KeyOf { target_type } => {
                self.type_requires_evaluative_normalization(ctx, *target_type)
            }
            _ => false,
        }
    }

    /// Return evaluative policy for normalizing one `keyof` operand.
    pub(crate) fn keyof_normalization_policy(
        &self,
        ctx: TypeView<'_>,
        operand_type_id: LocalTypeId,
        default_mode: NormalizationMode,
    ) -> (bool, NormalizationMode) {
        let needs_evaluation = self.type_requires_evaluative_normalization(ctx, operand_type_id);
        let normalize_mode = if needs_evaluation {
            NormalizationMode::Flow
        } else {
            default_mode
        };

        (needs_evaluation, normalize_mode)
    }

    /// Normalize decidable type operators into boolean literal types.
    pub(crate) fn normalize_decidable_type_operator(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        is_membership_check: bool,
        left: LocalTypeId,
        right: LocalTypeId,
        mode: NormalizationMode,
        _relation_mode: RelationMode,
    ) -> Option<LocalTypeId> {
        // type operators always use type operations semantics
        let relation_mode = RelationMode::TYPE_OPERATOR;

        // unwrap type values before assignability checks
        let unwrap_value = |ty_id: LocalTypeId, types: &TypeTable| match types.get_type(ty_id) {
            Type::Value { value } => *value,
            _ => ty_id,
        };
        let left = unwrap_value(left, ctx.types);
        let right = unwrap_value(right, ctx.types);

        // treat evaluation dependent checks as undecidable
        let left_needs_evaluation =
            self.type_requires_evaluative_normalization(ctx.type_view(), left);
        let right_needs_evaluation =
            self.type_requires_evaluative_normalization(ctx.type_view(), right);
        let is_decidable = !(left_needs_evaluation || right_needs_evaluation);

        // compute assignability for operator semantics
        let assignability = if is_membership_check {
            let mut key_visited = Vec::new();
            let key_type_id = self.normalize_keyof_type(
                &mut ctx.reborrow(),
                source_id,
                None,
                right,
                mode,
                relation_mode,
                &mut key_visited,
            );
            self.is_type_assignable(&mut ctx.reborrow(), key_type_id, left)
        } else {
            self.is_type_assignable(&mut ctx.reborrow(), right, left)
        };

        let ty = if !is_decidable {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            }
        } else {
            let value = matches!(assignability, Assignability::Assignable);
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(value)),
            }
        };
        Some(ctx.types.insert_type_from_any(ty, source_id))
    }
}
