use std::collections::HashSet;

use destack_dir::{
    LocalNodeIdAny, LocalTypeId, NormalizationMode, PrimitiveType, ScalarLiteral, SymbolTable,
    Type, TypeBinaryOperator, TypeLiteral, TypeTable, TypeUnaryOperator,
};
use destack_workspace::{Module, ProfileId};

use super::RelationMode;
use crate::{Assignability, Compiler};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check whether a type requires evaluative normalization.
    pub(crate) fn type_requires_evaluative_normalization(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // check for free static parameter references
        let mut static_visited = HashSet::new();
        let bound = HashSet::new();
        if self.type_contains_free_static_parameters(
            module,
            profile,
            type_id,
            &bound,
            symbols,
            types,
            &mut static_visited,
        ) {
            return true;
        }

        // check for inference variables
        let mut infer_visited = HashSet::new();
        if self.type_contains_infer_vars(type_id, types, &mut infer_visited) {
            return true;
        }

        // check for conditional infer bindings
        let mut binding_visited = HashSet::new();
        if self.type_contains_infer(type_id, types, &mut binding_visited) {
            return true;
        }

        false
    }

    /// Check whether a type requires convergence before static evaluation.
    pub(crate) fn type_requires_static_evaluation_convergence(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // evaluation dependent types are not stable yet
        if self.type_requires_evaluative_normalization(module, profile, type_id, symbols, types) {
            return true;
        }

        // unevaluated static arguments are not stable yet
        let mut static_argument_visited = HashSet::new();
        if self.type_has_unevaluated_static_arguments(type_id, types, &mut static_argument_visited)
        {
            return true;
        }

        false
    }

    /// Check whether a type is converged for static evaluation.
    pub(crate) fn type_is_converged_for_static_evaluation(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        !self.type_requires_static_evaluation_convergence(module, profile, type_id, symbols, types)
    }

    /// Check whether one normalized type stays as deferred `keyof` projection.
    pub(crate) fn type_is_deferred_keyof_projection(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        match types.get_type(type_id) {
            Type::Unary {
                operator: TypeUnaryOperator::Keyof,
                right,
            } => {
                self.type_requires_evaluative_normalization(module, profile, *right, symbols, types)
            }
            _ => false,
        }
    }

    /// Return evaluative policy for normalizing one `keyof` operand.
    pub(crate) fn keyof_normalization_policy(
        &self,
        module: &Module,
        profile: ProfileId,
        operand_type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
        default_mode: NormalizationMode,
    ) -> (bool, NormalizationMode) {
        let needs_evaluation = self.type_requires_evaluative_normalization(
            module,
            profile,
            operand_type_id,
            symbols,
            types,
        );
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
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        operator: TypeBinaryOperator,
        left: LocalTypeId,
        right: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        _relation_mode: RelationMode,
    ) -> Option<LocalTypeId> {
        // type operators always use type operations semantics
        let relation_mode = RelationMode::TYPE_OPERATOR;

        if !matches!(
            operator,
            TypeBinaryOperator::In
                | TypeBinaryOperator::Is
                | TypeBinaryOperator::InstanceOf
                | TypeBinaryOperator::Extends
                | TypeBinaryOperator::Implements
        ) {
            return None;
        }

        // unwrap type values before assignability checks
        let unwrap_value = |ty_id: LocalTypeId, types: &TypeTable| match types.get_type(ty_id) {
            Type::Value { value } => *value,
            _ => ty_id,
        };
        let left = unwrap_value(left, types);
        let right = unwrap_value(right, types);

        // treat evaluation dependent checks as undecidable
        let left_needs_evaluation =
            self.type_requires_evaluative_normalization(module, profile, left, symbols, types);
        let right_needs_evaluation =
            self.type_requires_evaluative_normalization(module, profile, right, symbols, types);
        let is_decidable = !(left_needs_evaluation || right_needs_evaluation);
        let options = self.analyze_context_options_for_module(module.id);

        // compute assignability for operator semantics
        let assignability = if operator == TypeBinaryOperator::In {
            let mut key_visited = Vec::new();
            let key_type_id = self.normalize_keyof_type(
                module,
                profile,
                source_id,
                None,
                right,
                symbols,
                types,
                mode,
                relation_mode,
                &mut key_visited,
            );
            self.is_type_assignable(module, profile, symbols, key_type_id, left, types, &options)
        } else {
            self.is_type_assignable(module, profile, symbols, right, left, types, &options)
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
        Some(types.insert_type_from_any(ty, source_id))
    }
}
