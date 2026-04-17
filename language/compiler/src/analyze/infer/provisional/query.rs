use crate::Compiler;
use destack_dir::{
    GlobalNodeIdAny, GlobalSymbolId, InferTable, Resolution, StaticArgument, TypeTable,
};

impl Compiler {
    /// Look up non-empty instance arguments attached to a node for an optional symbol.
    pub(crate) fn query_instance_arguments_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: Option<GlobalSymbolId>,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        let instance_id = types.get_instance_for_node(node_id)?;
        let instance = types.get_instance(instance_id);

        if let Some(symbol_id) = symbol_id
            && instance.symbol_id != symbol_id
        {
            return None;
        }
        if instance.generic_arguments.is_empty() {
            return None;
        }

        Some(instance.generic_arguments.clone())
    }

    /// Look up non-empty instance arguments attached to a node in infer state.
    pub(crate) fn query_instance_arguments_for_node_infer(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: Option<GlobalSymbolId>,
        infer: &InferTable,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        if let Some((_, arguments)) =
            self.query_instance_commit_obligation_for_node(node_id, symbol_id, infer)
        {
            return Some(arguments);
        }

        if let Some(instance_id) = infer.provisional_instance_for_node(node_id) {
            let instance = types.get_instance(instance_id);
            if let Some(symbol_id) = symbol_id
                && instance.symbol_id != symbol_id
            {
                return None;
            }
            if instance.generic_arguments.is_empty() {
                return None;
            }

            return Some(instance.generic_arguments.clone());
        }

        self.query_instance_arguments_for_node(node_id, symbol_id, types)
    }

    /// Look up non-empty instance symbol and arguments attached to a node in infer state.
    pub(crate) fn query_instance_symbol_arguments_for_node_infer(
        &self,
        node_id: GlobalNodeIdAny,
        infer: &InferTable,
        types: &TypeTable,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        if let Some(instance) = self.query_instance_commit_obligation_for_node(node_id, None, infer)
        {
            return Some(instance);
        }

        if let Some(instance_id) = infer.provisional_instance_for_node(node_id) {
            let instance = types.get_instance(instance_id);
            if instance.generic_arguments.is_empty() {
                return None;
            }

            return Some((instance.symbol_id, instance.generic_arguments.clone()));
        }

        let instance_id = types.get_instance_for_node(node_id)?;
        let instance = types.get_instance(instance_id);
        if instance.generic_arguments.is_empty() {
            return None;
        }

        Some((instance.symbol_id, instance.generic_arguments.clone()))
    }

    /// Look up one resolution attached to a node in infer state.
    pub(crate) fn query_resolution_for_node_infer<'a>(
        &self,
        node_id: GlobalNodeIdAny,
        infer: &'a InferTable,
        types: &'a TypeTable,
    ) -> Option<&'a Resolution> {
        if let Some(resolution) = infer.provisional_resolution_for_node(node_id) {
            return Some(resolution);
        }

        let resolution_id = types.get_resolution_for_node(node_id)?;
        Some(types.get_resolution(resolution_id))
    }

    /// Look up one instance-commit obligation attached to a node for an optional symbol.
    fn query_instance_commit_obligation_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: Option<GlobalSymbolId>,
        infer: &InferTable,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        let obligation_id = infer.instance_commit_obligation_id_for_node(node_id)?;
        let obligation = infer.instance_commit_obligation(obligation_id)?;

        if let Some(symbol_id) = symbol_id
            && obligation.symbol_id != symbol_id
        {
            return None;
        }
        if obligation.generic_arguments.is_empty() {
            return None;
        }

        Some((obligation.symbol_id, obligation.generic_arguments.clone()))
    }
}
