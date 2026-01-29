use std::collections::HashSet;

use destack_dir::{
    Expression, GlobalSymbolId, InferTable, LocalNodeId, LocalTypeId, Mutability, NodeTree,
    SymbolSpace, SymbolTable, Type, TypeField, TypeTable,
};
use destack_workspace::Module;

use super::ObjectShape;
use crate::{AnalyzeResult, Compiler, InferContext};

// allow wide signature for globalThis synthesis
#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Build the globalThis value type from global symbol bindings.
    pub(crate) fn infer_global_this_value_type(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        global_this_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // reuse object types that already exist
        if let Some(existing) = types.get_value_type_id(global_this_symbol)
            && matches!(types.get_type(existing), Type::Object { .. })
        {
            return Ok(Some(existing));
        }

        // locate the global symbol table for this module
        let global_table = self
            .program
            .index
            .global_symbol_tables
            .iter()
            .find_map(|entry| {
                let (key, table) = entry.pair();
                if key.profile_id == ctx.profile && table.module_versions.contains_key(&module.id) {
                    return Some(table.clone());
                }
                None
            });
        let Some(global_table) = global_table else {
            return Ok(None);
        };

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
                let value_ty_id = if let Some(value_ty_id) = types.get_value_type_id(*symbol_id) {
                    value_ty_id
                } else if symbol_id.module_id != module.id {
                    self.resolve_remote_symbol_value_type(
                        module,
                        ctx.profile,
                        expression_id.into_any(),
                        *symbol_id,
                        ctx.is_surface_inference,
                        types,
                    )?
                } else if let Some(inferred_ty_id) = self.infer_direct_binding_value_type(
                    module,
                    ctx.profile,
                    *symbol_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )? {
                    inferred_ty_id
                } else {
                    continue;
                };

                // compute readonly status from the binding mutability
                let binding_mutability = if symbol_id.module_id == module.id {
                    symbols.get_symbol(symbol_id.local_id).binding_mutability
                } else {
                    let remote_module = self.program.modules.get(symbol_id.module_id);
                    let remote_module = remote_module.read();
                    let remote_dir = remote_module.dir(ctx.profile);
                    let remote_symbols = remote_dir.symbols.read();
                    remote_symbols
                        .get_symbol(symbol_id.local_id)
                        .binding_mutability
                };
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
        let ty_id = types.insert_type_from_any(shape.into_object_type(), expression_id.into_any());
        types.set_value_type(global_this_symbol, ty_id);

        Ok(Some(ty_id))
    }
}
