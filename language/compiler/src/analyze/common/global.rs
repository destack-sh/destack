use std::collections::HashSet;

use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, Mutability, SymbolSpace, Type, TypeField,
};

use super::{InferContext, ObjectShape};
use crate::{AnalyzeError, AnalyzeResult, Compiler, InferState};

impl Compiler {
    /// Build the globalThis value type from global symbol bindings.
    pub(crate) fn infer_global_this_value_type(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        global_this_symbol: GlobalSymbolId,
        state: &InferState,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // reuse object types that already exist
        if let Some(existing) = ctx.types.get_value_type_id(global_this_symbol)
            && matches!(ctx.types.get_type(existing), Type::Object { .. })
        {
            return Ok(Some(existing));
        }

        // locate the global symbol table for this module
        let global_table = self
            .global_symbol_table_for_module(ctx.module.id, ctx.profile)
            .map_err(AnalyzeError::from)?;

        // collect global value bindings into a single shape
        let mut shape = ObjectShape::default();
        let mut seen = HashSet::new();
        for space in [SymbolSpace::Value, SymbolSpace::TypeValue] {
            for (group_key, symbol_id) in global_table.symbols_by_space.iter() {
                if group_key.space != space {
                    continue;
                }
                if !seen.insert(group_key.key) {
                    continue;
                }
                if *symbol_id == global_this_symbol {
                    continue;
                }
                if !self.symbol_is_value_capable(ctx.profile, *symbol_id) {
                    continue;
                }

                // resolve the value type for the global binding
                let value_ty_id = if let Some(value_ty_id) = ctx.types.get_value_type_id(*symbol_id)
                {
                    value_ty_id
                } else if symbol_id.module_id != ctx.module.id {
                    self.resolve_remote_symbol_value_type_for_context(
                        &mut ctx.reborrow(),
                        state,
                        expression_id.into_any(),
                        *symbol_id,
                    )?
                } else if let Some(inferred_ty_id) =
                    self.infer_direct_binding_value_type(&mut ctx.reborrow(), *symbol_id, state)?
                {
                    inferred_ty_id
                } else {
                    continue;
                };

                // compute readonly status from the binding mutability
                let binding_mutability = self
                    .with_module_symbols_or_local_for_artifact(
                        ctx.module,
                        ctx.profile,
                        symbol_id.module_id,
                        ctx.symbols,
                        destack_workspace::ArtifactKey::dir_declared,
                        |_, owner_symbols| {
                            owner_symbols
                                .get_symbol(symbol_id.local_id)
                                .binding_mutability
                        },
                    )
                    .map_err(AnalyzeError::from)?;
                let is_readonly = matches!(binding_mutability, Some(Mutability::Immutable));

                shape.fields.push(TypeField {
                    key: group_key.key,
                    ty: value_ty_id,
                    is_optional: false,
                    is_readonly,
                });
            }
        }

        // register the synthesized globalThis type
        let ty_id = ctx
            .types
            .insert_type_from_any(shape.into_object_type(), expression_id.into_any());
        ctx.types.set_value_type(global_this_symbol, ty_id);

        Ok(Some(ty_id))
    }
}
