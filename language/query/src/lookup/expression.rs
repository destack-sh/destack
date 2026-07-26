use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};

use crate::ModuleQueryContext;

impl ModuleQueryContext<'_> {
    /// Return every symbol target from one recorded use-site resolution.
    pub(crate) fn recorded_symbol_targets(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> Option<Vec<dir::GlobalSymbolId>> {
        // member resolutions distinguish declaration-backed and structural access
        if let Some(symbols) = self.member_symbol_targets(node_id) {
            return Some(symbols);
        }

        // explicit instantiations record their selected declaration directly
        if let Some(resolution) = self.resolutions().instantiation_resolution(node_id) {
            return Some(vec![resolution.symbol]);
        }

        // lexical and path resolutions retain every selected declaration
        if let Some(symbols) = self.name_symbol_targets(node_id) {
            return Some(symbols);
        }

        // receiver expressions record the declaration introducing the receiver
        if let Some(resolution) = self.resolutions().receiver_resolution(node_id) {
            return Some(vec![resolution.declaration]);
        }

        if let Some(resolution) = self.resolutions().label_resolution(node_id) {
            let symbols = match resolution {
                dir::LabelResolution::Symbol(symbol) => vec![symbol],
                dir::LabelResolution::Loop | dir::LabelResolution::Function => Vec::new(),
            };

            return Some(symbols);
        }

        None
    }

    /// Return every symbol target from one recorded name resolution.
    fn name_symbol_targets(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> Option<Vec<dir::GlobalSymbolId>> {
        let resolution = self.resolutions().name_resolution(node_id)?;

        let symbols = resolution.symbols().to_vec();

        Some(symbols)
    }

    /// Return the recorded symbol targets for one dependency item.
    pub(crate) fn dependency_symbol_targets(
        &self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> ProviderResult<Vec<dir::GlobalSymbolId>> {
        let targets = self.dependency_targets(item_id)?;
        let symbols = targets
            .into_iter()
            .filter_map(|target| match target {
                dir::ImportTarget::Symbol(symbol_id) => Some(symbol_id),
                dir::ImportTarget::Namespace(_) => None,
            })
            .collect();

        Ok(symbols)
    }

    /// Return every recorded target for one dependency item.
    pub(crate) fn dependency_targets(
        &self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> ProviderResult<Vec<dir::ImportTarget>> {
        let source = item_id.into_global_any(self.module_id());
        let reference = self.resolved().references.get(source).ok_or_else(|| {
            ProviderError::internal(format!(
                "dependency item has no resolved reference: {source:?}"
            ))
        })?;

        let targets = match reference {
            dir::Reference::Bound(symbols) => symbols
                .iter()
                .copied()
                .map(dir::ImportTarget::Symbol)
                .collect(),
            dir::Reference::Namespace(module) => vec![dir::ImportTarget::Namespace(*module)],
            dir::Reference::Projected { .. } => {
                return Err(ProviderError::internal(format!(
                    "dependency item has a projected reference: {source:?}"
                ))
                .into());
            }
            dir::Reference::Ambiguous(targets) => targets.to_vec(),
            dir::Reference::Missing => Vec::new(),
        };

        Ok(targets)
    }

    /// Return the local symbol introduced by one dependency item.
    pub(crate) fn dependency_local_symbol(
        &self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> Option<dir::GlobalSymbolId> {
        self.global_node_symbol(item_id.into())
    }
}
