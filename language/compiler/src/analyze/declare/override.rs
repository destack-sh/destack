use std::collections::HashMap;

use crate::Compiler;
use crate::analyze::common::{CanonicalSymbolMode, SymbolTypeView, TypeContext};
use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, StaticKey, Type, TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check whether a member overrides a base-chain member.
    pub(crate) fn member_overrides_base_chain(
        &self,
        base_symbol: Option<GlobalSymbolId>,
        is_static: bool,
        member_key: &StaticKey,
        types: &TypeTable,
    ) -> bool {
        self.base_chain_member_type(base_symbol, is_static, member_key, types)
            .is_some()
    }

    /// Resolve one member type from the nearest symbol in the base chain.
    pub(crate) fn base_chain_member_type(
        &self,
        mut base_symbol: Option<GlobalSymbolId>,
        is_static: bool,
        member_key: &StaticKey,
        types: &TypeTable,
    ) -> Option<(GlobalSymbolId, LocalTypeId)> {
        while let Some(symbol) = base_symbol {
            if let Some(member_ty_id) =
                self.symbol_member_type_for_key(symbol, is_static, member_key, types)
            {
                return Some((symbol, member_ty_id));
            }

            base_symbol = types
                .get_lineage_for_symbol(symbol)
                .and_then(|lineage| lineage.extends);
        }

        None
    }

    /// Build direct-base substitutions from the class extends clause.
    pub(crate) fn direct_base_substitutions(
        &self,
        ctx: &mut TypeContext<'_>,
        base_symbol: Option<GlobalSymbolId>,
        extends_expression: Option<LocalNodeId<Expression>>,
    ) -> HashMap<GlobalSymbolId, LocalTypeId> {
        let Some(base_symbol) = base_symbol else {
            return HashMap::new();
        };
        let Some(extends_expression_id) = extends_expression else {
            return HashMap::new();
        };

        // resolve the base heritage reference
        let extends_expression = ctx.tree.get(extends_expression_id);
        let Some(resolved_symbol) = extends_expression.target_symbol() else {
            return HashMap::new();
        };
        let evaluated_static_arguments = match self
            .evaluate_generic_arguments(&mut ctx.reborrow(), extends_expression.generic_arguments())
        {
            Ok(arguments) => arguments.unwrap_or_default(),
            Err(_) => return HashMap::new(),
        };
        let resolved_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            resolved_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        if resolved_symbol != base_symbol {
            return HashMap::new();
        }
        let static_arguments = match self.resolve_declared_type_reference_static_arguments(
            &mut ctx.reborrow(),
            extends_expression_id.into_any(),
            base_symbol,
            Some(evaluated_static_arguments.as_slice()),
            true,
        ) {
            Ok(arguments) => arguments.unwrap_or(evaluated_static_arguments),
            Err(_) => return HashMap::new(),
        };

        self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.reborrow(),
            base_symbol,
            extends_expression_id.into_any(),
            &static_arguments,
        )
    }

    /// Resolve one member type from a symbol object shape by key.
    pub(crate) fn symbol_member_type_for_key(
        &self,
        symbol: GlobalSymbolId,
        is_static: bool,
        member_key: &StaticKey,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // resolve the correct side for member lookup
        let type_id = if is_static {
            types.get_value_type_id(symbol)
        } else {
            types.get_instance_type_id(symbol)
        };
        let type_id = type_id?;

        // only object types expose fields for override checks
        let Type::Object { fields, .. } = types.get_type(type_id) else {
            return None;
        };
        fields
            .iter()
            .find(|field| field.key.matches(member_key))
            .map(|field| field.ty)
    }

    /// Normalize one override member type before compatibility checks.
    pub(crate) fn normalize_override_signature_type(
        &self,
        type_id: LocalTypeId,
        ctx: &mut TypeContext<'_>,
        dropped_parameter_symbols: &[GlobalSymbolId],
    ) -> LocalTypeId {
        let Type::Function {
            asynchrony,
            cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        } = ctx.types.get_type(type_id).clone()
        else {
            return type_id;
        };

        let static_parameters = static_parameters
            .into_iter()
            .filter(|parameter_type_id| {
                if let Type::Reference { symbol, .. } = ctx.types.get_type(*parameter_type_id)
                    && dropped_parameter_symbols.contains(symbol)
                {
                    return false;
                }

                self.signature_parameter_is_unresolved_static(
                    *parameter_type_id,
                    ctx.symbol_type_view(),
                    ctx.types,
                )
            })
            .collect::<Vec<_>>();
        let normalized = Type::Function {
            asynchrony,
            cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        };
        ctx.types.insert_type_from_type(normalized, type_id)
    }

    /// Return true when one signature static parameter remains an unresolved static reference.
    fn signature_parameter_is_unresolved_static(
        &self,
        parameter_type_id: LocalTypeId,
        view: SymbolTypeView<'_>,
        types: &TypeTable,
    ) -> bool {
        let Type::Reference { symbol, .. } = types.get_type(parameter_type_id) else {
            return false;
        };
        self.symbol_is_static_parameter(view, *symbol)
    }
}
