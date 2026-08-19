use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return the strongest access granted through one checked place.
    pub(crate) fn place_access(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<dir::Access>, ProviderError> {
        let node = node.into_global(self.id);
        let Some(place) = self.decisions.place_resolution(node) else {
            return Ok(None);
        };

        self.dir.memory_access(place.access).map(Some)
    }

    /// Return the stable storage selected by one checked expression.
    pub fn access_resolution(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::AccessResolution> {
        let global = node.into_global_any(self.id);

        self.decisions.access_resolution(global)
    }

    /// Return the recorded uses of one stable access and its projections within a node.
    pub(crate) fn access_uses_within(
        &self,
        path: &dir::AccessPath,
        node: dir::LocalNodeIdAny,
        occurrences: &[dir::AccessOccurrence],
    ) -> dir::BindingUse {
        let view = self.view();
        let mut uses = dir::BindingUse::default();

        // merge uses of the selected storage beneath the selected node
        for occurrence in occurrences {
            if occurrence.path.starts_with(path) && view.is_inside(occurrence.node, node) {
                uses |= occurrence.uses;
            }
        }

        uses
    }

    /// Return the weakest access sufficient for one binding's checked uses.
    pub(crate) fn weakest_binding_access(
        &self,
        symbol: dir::GlobalSymbolId,
        within: dir::LocalNodeIdAny,
        occurrences: &[dir::AccessOccurrence],
    ) -> Result<dir::Access, ProviderError> {
        let view = self.view();
        let root = dir::AccessPath::symbol(symbol);
        let mut required = dir::Access::Readonly;

        // combine access requirements beneath the binding storage
        for occurrence in occurrences {
            if !view.is_inside(occurrence.node, within) || !occurrence.path.starts_with(&root) {
                continue;
            }

            // replacing the binding root requires exclusive access
            let access = if occurrence.uses.contains(dir::BindingUse::WRITTEN) {
                match occurrence.path == root {
                    true => dir::Access::Exclusive,
                    false => dir::Access::Mutable,
                }
            }
            // preserve access selected by checked reborrows
            else if occurrence.uses.contains(dir::BindingUse::MUTABLE) {
                let type_id = self.adjusted_type_id(occurrence.node)?;
                match self.dir.borrow_access(type_id)? {
                    Some(access) => access,
                    None => dir::Access::Mutable,
                }
            }
            // reads require only readonly access
            else {
                dir::Access::Readonly
            };
            if access.grants(required) {
                required = access;
            }
        }

        Ok(required)
    }
}
