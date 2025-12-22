use crate::{Compiler, InferOrigin, InferScope, InferTable};
use destack_dir::{GlobalNodeIdAny, GlobalSymbolId, LocalTypeId, Type, TypeTable};

impl Compiler {
    /// Get or create an inference variable type for a symbol.
    pub(super) fn infer_var_type_for_symbol(
        &self,
        infer: &mut InferTable,
        types: &mut TypeTable,
        symbol: GlobalSymbolId,
        origin: InferOrigin,
        scope: InferScope,
    ) -> LocalTypeId {
        // reuse existing inference variable when available
        if let Some(var_id) = infer.var_by_symbol_id.get(&symbol).copied()
            && let Some(ty_id) = infer.type_for_var(var_id)
            && ty_id.0 != u32::MAX
        {
            return ty_id;
        }

        // allocate and bind a new inference variable
        let var_id = infer.new_var(origin, scope);
        let ty_id = types.insert_type(Type::InferVar { id: var_id });
        infer.bind_type(var_id, ty_id);
        infer.var_by_symbol_id.insert(symbol, var_id);
        ty_id
    }

    /// Get or create an inference variable type for a node.
    pub(super) fn infer_var_type_for_node(
        &self,
        infer: &mut InferTable,
        types: &mut TypeTable,
        node_id: GlobalNodeIdAny,
        origin: InferOrigin,
        scope: InferScope,
    ) -> LocalTypeId {
        // reuse existing inference variable when available
        if let Some(var_id) = infer.var_by_node_id.get(&node_id).copied()
            && let Some(ty_id) = infer.type_for_var(var_id)
            && ty_id.0 != u32::MAX
        {
            return ty_id;
        }

        // allocate and bind a new inference variable
        let var_id = infer.new_var(origin, scope);
        let ty_id = types.insert_type(Type::InferVar { id: var_id });
        infer.bind_type(var_id, ty_id);
        infer.var_by_node_id.insert(node_id, var_id);
        ty_id
    }

    /// Check whether a type id points at an inference variable.
    pub(super) fn is_infer_var_type(&self, ty_id: LocalTypeId, types: &TypeTable) -> bool {
        matches!(types.get_type(ty_id), Type::InferVar { .. })
    }
}
