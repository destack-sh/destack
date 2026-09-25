use crate::EmitError;
use crate::emit::js::{DependencyBinding, DependencyItem, LocalNodeId, ModuleLowerer};
use tspp_dir as dir;
use tspp_source::ModuleId;

impl ModuleLowerer<'_> {
    /// Return the concrete module target for one dependency node.
    pub(crate) fn dependency_target_module(
        &self,
        source_id: dir::LocalNodeIdAny,
    ) -> Option<ModuleId> {
        let source = source_id.into_global(self.module.id);

        self.modules
            .target_for_source(source, dir::ModuleRelation::Import)
            .or_else(|| {
                self.modules
                    .target_for_source(source, dir::ModuleRelation::ReExport)
            })
    }

    /// Lower a dependency binding from DIR into JavaScript.
    pub(crate) fn lower_dependency_binding(
        &self,
        binding: dir::DependencyBinding,
    ) -> DependencyBinding {
        match binding {
            dir::DependencyBinding::Named => DependencyBinding::Named,
            dir::DependencyBinding::Default => DependencyBinding::Default,
            dir::DependencyBinding::Namespace => DependencyBinding::Namespace,
        }
    }

    /// Lower dependency items from DIR into JavaScript.
    pub(crate) fn lower_dependency_items(
        &mut self,
        item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> Result<Vec<LocalNodeId<DependencyItem>>, EmitError> {
        let mut lowered_item_ids: Vec<LocalNodeId<DependencyItem>> = Vec::new();
        for item_id in item_ids {
            let item = self.dir_tree.get(*item_id);
            match item {
                dir::DependencyItem::Error => {
                    return Err(self.unhandled(
                        item_id.into_global_any(self.module.id),
                        Some("dependency error slots are not lowered to JS".to_string()),
                    ));
                }
                dir::DependencyItem::Binding {
                    binding,
                    name,
                    alias,
                    value,
                } => {
                    if value.is_some() {
                        return Err(self.unhandled(
                            item_id.into_global_any(self.module.id),
                            Some("export assignment has no JavaScript module form".to_string()),
                        ));
                    }

                    let source_id = *item_id;
                    let binding = self.lower_dependency_binding(*binding);
                    let name = name.map(|name| self.lower_name(name));
                    let alias = *alias;
                    let item = DependencyItem {
                        binding,
                        name,
                        alias,
                    };
                    let item_id = self
                        .tree
                        .insert_from_source(item, self.module.id, source_id);

                    self.copy_source_node_symbol(item_id, source_id);

                    lowered_item_ids.push(item_id);
                }
            }
        }

        Ok(lowered_item_ids)
    }
}
