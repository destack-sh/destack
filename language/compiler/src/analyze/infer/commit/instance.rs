use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GlobalNodeIdAny, GlobalSymbolId, Instance, LocalInstanceId, LocalNodeId,
    LocalTypeId, NodeTree, NodeType, StaticArgument, SymbolTable, SymbolType, TypeTable,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Whether a symbol is instantiable (i.e. can have an instance type).
    pub(crate) fn symbol_is_instantiable(&self, symbol: GlobalSymbolId) -> bool {
        matches!(
            symbol.ty(),
            SymbolType::Class
                | SymbolType::Struct
                | SymbolType::Interface
                | SymbolType::Enum
                | SymbolType::Extension
                | SymbolType::TypeAlias
                | SymbolType::Newtype
        )
    }

    /// Commit instance facts for reference types that carry static arguments.
    pub(crate) fn commit_reference_instances(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let type_count = types.type_count();
        let options = self.analyze_context_options_for_module(module.id);
        // NOTE #Performance: this scans the full type table each infer pass
        for id in 0..type_count {
            let ty_id = LocalTypeId::new(id);

            // skip non reference types
            let Some((symbol, static_arguments, source_id)) = self.unwrap_type_symbol(types, ty_id)
            else {
                continue;
            };

            // skip non expression or annotation nodes
            if !matches!(source_id.ty, NodeType::Expression | NodeType::Annotation) {
                continue;
            }
            if !self.symbol_is_instantiable(symbol) {
                continue;
            }

            if let Some(resolved) = self.resolve_type_reference_static_arguments(
                module,
                profile,
                source_id,
                symbol,
                static_arguments.as_deref(),
                false,
                &options,
                tree,
                symbols,
                types,
            )? && !resolved.is_empty()
            {
                self.commit_instance_for_node(
                    source_id.into_global(module.id),
                    symbol,
                    resolved,
                    types,
                );
            }
        }

        Ok(())
    }

    /// Look up an existing instance id for a symbol and resolved arguments.
    pub(crate) fn query_instance_for_symbol_arguments(
        &self,
        symbol_id: GlobalSymbolId,
        static_arguments: &[StaticArgument],
        types: &TypeTable,
    ) -> Option<LocalInstanceId> {
        types.find_instance(symbol_id, static_arguments)
    }

    /// Look up an existing instance id attached to a node.
    pub(crate) fn query_instance_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        types: &TypeTable,
    ) -> Option<LocalInstanceId> {
        types.get_instance_for_node(node_id)
    }

    /// Look up non-empty instance arguments attached to a node for an optional symbol.
    pub(crate) fn query_instance_arguments_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: Option<GlobalSymbolId>,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        let instance_id = self.query_instance_for_node(node_id, types)?;
        let instance = types.get_instance(instance_id);

        if let Some(symbol_id) = symbol_id
            && instance.symbol_id != symbol_id
        {
            return None;
        }
        if instance.static_arguments.is_empty() {
            return None;
        }

        Some(instance.static_arguments.clone())
    }

    /// Look up non-empty instance symbol and arguments attached to a node.
    pub(crate) fn query_instance_symbol_arguments_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        types: &TypeTable,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        let instance_id = self.query_instance_for_node(node_id, types)?;
        let instance = types.get_instance(instance_id);
        if instance.static_arguments.is_empty() {
            return None;
        }

        Some((instance.symbol_id, instance.static_arguments.clone()))
    }

    /// Infer base instance arguments for member resolution from inherited or extension context.
    pub(crate) fn infer_member_instance_base_arguments(
        &self,
        inherited_arguments: &[StaticArgument],
        extension_arguments: Option<&[StaticArgument]>,
    ) -> Vec<StaticArgument> {
        match extension_arguments {
            Some(arguments) => arguments.to_vec(),
            None => inherited_arguments.to_vec(),
        }
    }

    /// Infer full instance arguments by appending resolved static arguments to a base vector.
    pub(crate) fn infer_instance_arguments_with_suffix(
        &self,
        mut base_arguments: Vec<StaticArgument>,
        resolved_arguments: &[StaticArgument],
    ) -> Vec<StaticArgument> {
        base_arguments.extend_from_slice(resolved_arguments);
        base_arguments
    }

    /// Commit an instance fact for a symbol and resolved arguments.
    pub(crate) fn commit_instance_for_symbol_arguments(
        &self,
        symbol_id: GlobalSymbolId,
        static_arguments: Vec<StaticArgument>,
        types: &mut TypeTable,
    ) -> LocalInstanceId {
        if let Some(existing) =
            self.query_instance_for_symbol_arguments(symbol_id, &static_arguments, types)
        {
            return existing;
        }

        let instance = Instance::new(symbol_id, static_arguments);
        types.insert_instance(instance)
    }

    /// Commit an instance fact for a symbol when arguments are non-empty.
    pub(crate) fn commit_instance_for_symbol_if_arguments(
        &self,
        symbol_id: GlobalSymbolId,
        static_arguments: Vec<StaticArgument>,
        types: &mut TypeTable,
    ) -> Option<LocalInstanceId> {
        if static_arguments.is_empty() {
            return None;
        }

        Some(self.commit_instance_for_symbol_arguments(symbol_id, static_arguments, types))
    }

    /// Commit an instance fact for a node.
    pub(crate) fn commit_instance_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        static_arguments: Vec<StaticArgument>,
        types: &mut TypeTable,
    ) -> LocalInstanceId {
        let instance_id =
            self.commit_instance_for_symbol_arguments(symbol_id, static_arguments, types);
        types.set_instance_for_node(node_id, instance_id);
        instance_id
    }

    /// Commit an instance fact for a node when arguments are non-empty.
    pub(crate) fn commit_instance_for_node_if_arguments(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        static_arguments: Vec<StaticArgument>,
        types: &mut TypeTable,
    ) -> Option<LocalInstanceId> {
        if static_arguments.is_empty() {
            return None;
        }

        Some(self.commit_instance_for_node(node_id, symbol_id, static_arguments, types))
    }

    /// Collect instance arguments recorded on a member expression.
    pub(crate) fn query_member_instance_arguments_for_call(
        &self,
        module: &Module,
        member_expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        let member_symbol = member_symbol?;
        self.query_instance_arguments_for_node(
            member_expression_id.into_global_any(module.id),
            Some(member_symbol),
            types,
        )
    }
}
