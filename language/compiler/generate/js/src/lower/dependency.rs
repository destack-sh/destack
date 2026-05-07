use crate::{
    CodegenJsError, CodegenJsResult, CodegenJsResultExt, DependencyBinding, DependencyItem,
    DependencySpace, Expression, LocalNodeId, ModuleLowerer,
};
use destack_dir as dir;
use destack_source::ModuleId;

impl ModuleLowerer<'_> {
    /// Return the concrete module target for one dependency node.
    pub(crate) fn dependency_target_module(
        &self,
        source_id: dir::LocalNodeIdAny,
        space: dir::DependencySpace,
    ) -> Option<ModuleId> {
        let resolution = self
            .types
            .dependency_resolution(source_id.into_global(self.module.id))?;

        // dependency bindings are tracked on individual import or export items
        let dir::DependencyResolution::Module(resolution) = resolution else {
            return None;
        };

        // external targets are preserved as bare specifiers for the linker
        let dir::ModuleTarget::Module(module_id) = resolution.for_space(space)? else {
            return None;
        };

        Some(module_id)
    }

    /// Lower a dependency space from DIR into JS AST.
    pub fn lower_dependency_space(&self, space: dir::DependencySpace) -> DependencySpace {
        match space {
            dir::DependencySpace::Type => DependencySpace::Type,
            dir::DependencySpace::Value => DependencySpace::Value,
        }
    }

    /// Lower a dependency binding from DIR into JS AST.
    pub fn lower_dependency_binding(&self, binding: dir::DependencyBinding) -> DependencyBinding {
        match binding {
            dir::DependencyBinding::Item => DependencyBinding::Item,
            dir::DependencyBinding::Default => DependencyBinding::Default,
            dir::DependencyBinding::Namespace => DependencyBinding::Namespace,
        }
    }

    /// Lower dependency items from DIR into JS AST.
    pub fn lower_dependency_items(
        &mut self,
        space: dir::DependencySpace,
        item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CodegenJsResult<Vec<LocalNodeId<DependencyItem>>> {
        let mut lowered_item_ids: Vec<LocalNodeId<DependencyItem>> = Vec::new();
        for item_id in item_ids {
            let item = self.dir_tree.get(*item_id);
            let lowered_item = match item {
                dir::DependencyItem::Error => {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: item_id.into_global_any(self.module.id),
                        message: Some("dependency error slots are not lowered to JS".to_string()),
                    });
                }
                dir::DependencyItem::Item {
                    binding,
                    space: item_kind,
                    name,
                    alias,
                    symbol,
                } => {
                    let source_id = *item_id;
                    let binding = self.lower_dependency_binding(*binding);
                    let name = name.map(|name| self.lower_name(name));
                    let alias = *alias;
                    let item = DependencyItem {
                        binding,
                        space: if *item_kind != space {
                            Some(self.lower_dependency_space(*item_kind))
                        } else {
                            None
                        },
                        name,
                        alias,
                        value: None,
                    };
                    let item_id = self
                        .tree
                        .insert_from_source(item, self.module.id, source_id);

                    // resolved imported/exported bindings use the target symbol
                    let resolution = self
                        .types
                        .dependency_resolution(source_id.into_global_any(self.module.id));
                    if let Some(dir::DependencyResolution::Binding(target_symbol)) = resolution {
                        self.set_global_node_symbol(item_id, *target_symbol);
                    }
                    // local declaration items keep their source symbol
                    else if let Some(symbol) = symbol {
                        self.set_source_node_symbol(item_id, *symbol);
                    }

                    lowered_item_ids.push(item_id);
                    continue;
                }
                dir::DependencyItem::Value { binding, value } => {
                    let binding = self.lower_dependency_binding(*binding);
                    let value_id = self
                        .lower_expression(*value)
                        .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;
                    DependencyItem {
                        binding,
                        space: None,
                        name: None,
                        alias: None,
                        value: Some(value_id),
                    }
                }
            };
            let item_id = self
                .tree
                .insert_from_source(lowered_item, self.module.id, *item_id);
            lowered_item_ids.push(item_id);
        }
        Ok(lowered_item_ids)
    }
}
