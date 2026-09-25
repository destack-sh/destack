use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::WalkState;

impl WalkState<'_, '_> {
    /// Walk one dependency item.
    ///
    /// Example:
    /// ```tspp
    /// export const value: number = 1;
    /// ```
    pub(in crate::sema) fn walk_dependency_item(
        &mut self,
        id: dir::LocalNodeId<dir::DependencyItem>,
        dependency_item: &dir::DependencyItem,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }

        match dependency_item {
            // import { name: value }
            dir::DependencyItem::Binding {
                value: Some(value), ..
            } => {
                // check dependency alias in declaration context
                let before_value = self.fork_flow();
                self.walk_expression(*value, self.tree.get(*value))?;
                self.restore_flow(before_value);
            }
            // import { name }
            dir::DependencyItem::Binding { value: None, .. } => {}
            // ignore damaged nodes
            dir::DependencyItem::Error => {}
        };

        Ok(())
    }
}
