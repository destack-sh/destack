use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GlobalNodeIdAny, GlobalSymbolId, Instance, LocalInstanceId, LocalNodeId,
    LocalTypeId, NodeTree, NodeType, StaticArgument, SymbolTable, SymbolType, Type, TypeTable,
};
use destack_workspace::Module;

impl Compiler {
    /// Whether a symbol is instantiable (i.e. can have an instance type).
    pub(super) fn is_instantiable_symbol(&self, symbol: GlobalSymbolId) -> bool {
        matches!(
            symbol.ty(),
            SymbolType::Class
                | SymbolType::Struct
                | SymbolType::Interface
                | SymbolType::Enum
                | SymbolType::TypeAlias
                | SymbolType::Newtype
        )
    }

    /// Register instances for type references.
    pub(super) fn register_instances(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let type_count = types.type_count();
        for id in 0..type_count {
            let ty_id = LocalTypeId::new(id);
            let ty = types.get_type(ty_id).clone();
            let Type::Reference {
                symbol,
                static_arguments,
            } = ty
            else {
                continue;
            };

            let source_id = types.get_type_source(ty_id);
            if source_id.id == u32::MAX || source_id.ty != NodeType::Expression {
                continue;
            }
            if !self.is_instantiable_symbol(symbol) {
                continue;
            }

            if let Some(resolved) = self.resolve_type_reference_static_arguments(
                module,
                source_id,
                symbol,
                static_arguments.as_deref(),
                tree,
                symbols,
                types,
            )? && !resolved.is_empty()
            {
                self.register_instance_for_node(
                    source_id.into_global(module.id),
                    symbol,
                    resolved,
                    types,
                );
            }
        }

        Ok(())
    }

    /// Record an Instance for a node.
    pub(super) fn register_instance_for_node(
        &self,
        node_id: GlobalNodeIdAny,
        symbol_id: GlobalSymbolId,
        static_arguments: Vec<StaticArgument>,
        types: &mut TypeTable,
    ) -> LocalInstanceId {
        let instance_id = self.register_instance_for_symbol(symbol_id, static_arguments, types);
        types.set_instance_for_node(node_id, instance_id);
        instance_id
    }

    /// Register or reuse an Instance for a symbol.
    pub(super) fn register_instance_for_symbol(
        &self,
        symbol_id: GlobalSymbolId,
        static_arguments: Vec<StaticArgument>,
        types: &mut TypeTable,
    ) -> LocalInstanceId {
        if let Some(existing) = types.find_instance(symbol_id, &static_arguments) {
            return existing;
        }

        let instance = Instance::new(symbol_id, static_arguments);
        types.insert_instance(instance)
    }

    /// Collect instance arguments recorded on a member expression.
    pub(super) fn member_instance_arguments_for_call(
        &self,
        module: &Module,
        member_expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        let member_symbol = member_symbol?;

        // load the instance attached to the member expression
        let instance_id =
            types.get_instance_for_node(member_expression_id.into_global_any(module.id))?;
        let instance = types.get_instance(instance_id);

        if instance.symbol_id != member_symbol || instance.static_arguments.is_empty() {
            return None;
        }

        Some(instance.static_arguments.clone())
    }
}
