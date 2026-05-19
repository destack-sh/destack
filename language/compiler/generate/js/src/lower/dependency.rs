use crate::{
    CodegenJsError, CodegenJsResult, CodegenJsResultExt, DependencyBinding, DependencyForm,
    DependencyItem, Expression, LocalNodeId, ModuleLowerer,
};
use destack_dir as dir;
use destack_source::ModuleId;

impl ModuleLowerer<'_> {
    /// Return the concrete module target for one dependency node.
    pub(crate) fn dependency_target_module(
        &self,
        source_id: dir::LocalNodeIdAny,
    ) -> Option<ModuleId> {
        let source = source_id.into_global(self.module.id);

        self.dependencies
            .target_for_source(source, dir::DependencyRelation::Import)
            .or_else(|| {
                self.dependencies
                    .target_for_source(source, dir::DependencyRelation::ReExport)
            })
    }

    /// Lower a dependency form from DIR into JS AST.
    pub fn lower_dependency_form(&self, form: dir::DependencyForm) -> DependencyForm {
        match form {
            dir::DependencyForm::Type => DependencyForm::Type,
            dir::DependencyForm::Plain => DependencyForm::Plain,
        }
    }

    /// Lower a dependency binding from DIR into JS AST.
    pub fn lower_dependency_binding(&self, binding: dir::DependencyBinding) -> DependencyBinding {
        match binding {
            dir::DependencyBinding::Named => DependencyBinding::Named,
            dir::DependencyBinding::Default => DependencyBinding::Default,
            dir::DependencyBinding::Namespace => DependencyBinding::Namespace,
        }
    }

    /// Lower dependency items from DIR into JS AST.
    pub fn lower_dependency_items(
        &mut self,
        form: dir::DependencyForm,
        item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CodegenJsResult<Vec<LocalNodeId<DependencyItem>>> {
        let mut lowered_item_ids: Vec<LocalNodeId<DependencyItem>> = Vec::new();
        for item_id in item_ids {
            let item = self.dir_tree.get(*item_id);
            match item {
                dir::DependencyItem::Error => {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: item_id.into_global_any(self.module.id),
                        message: Some("dependency error slots are not lowered to JS".to_string()),
                    });
                }
                dir::DependencyItem::Binding {
                    binding,
                    form: item_form,
                    name,
                    alias,
                    value,
                } => {
                    let source_id = *item_id;
                    let binding = self.lower_dependency_binding(*binding);
                    let name = name.map(|name| self.lower_name(name));
                    let alias = *alias;
                    let value = value
                        .map(|value| {
                            self.lower_expression(value).expect_node::<Expression>(
                                value.into_global_any(self.module.id),
                                self,
                            )
                        })
                        .transpose()?;
                    let item_form = item_form.unwrap_or(form);
                    let item = DependencyItem {
                        binding,
                        form: if item_form != form {
                            Some(self.lower_dependency_form(item_form))
                        } else {
                            None
                        },
                        name,
                        alias,
                        value,
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
