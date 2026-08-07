use destack_dir as dir;

use crate::{ModuleQueryContext, QueryError, QueryResult};

impl ModuleQueryContext<'_> {
    /// Return every symbol target from one recorded use-site resolution.
    pub(crate) fn symbol_targets(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> QueryResult<Option<Vec<dir::GlobalSymbolId>>> {
        let mut selections = Vec::new();

        // collect every exact symbol selection recorded for this node
        if let Some(resolution) = self.resolutions()?.member_resolution(node_id) {
            selections.push(resolution.target_symbols());
        }
        if let Some(resolution) = self.resolutions()?.subscript_resolution(node_id) {
            selections.push(resolution.target_symbols());
        }
        if let Some(resolution) = self.resolutions()?.instantiation_resolution(node_id) {
            selections.push(vec![resolution.symbol]);
        }
        if let Some(resolution) = self.resolutions()?.name_resolution(node_id) {
            selections.push(resolution.symbols().to_vec());
        }
        if let Some(resolution) = self.resolutions()?.receiver_resolution(node_id) {
            selections.push(vec![resolution.declaration]);
        }
        if let Some(symbol) = self.resolutions()?.label_resolution(node_id) {
            selections.push(vec![symbol]);
        }

        // require one authoritative resolution column
        if selections.len() > 1 {
            return Err(QueryError::conflict(format!(
                "symbol resolution columns: {node_id:?}"
            )));
        }

        Ok(selections.pop())
    }

    /// Return the recorded symbol targets for one dependency item.
    pub(crate) fn dependency_symbol_targets(
        &self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
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
    ) -> QueryResult<Vec<dir::ImportTarget>> {
        let source = item_id.into_global_any(self.module_id());
        let reference = self.resolved()?.references.get(source).ok_or_else(|| {
            QueryError::missing(format!(
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
                return Err(QueryError::invalid(format!(
                    "dependency item has a projected reference: {source:?}"
                )));
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
    ) -> QueryResult<Option<dir::GlobalSymbolId>> {
        self.global_node_symbol(item_id.into())
    }
}
