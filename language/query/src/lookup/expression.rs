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
        if let Some(resolution) = self.decisions()?.member_decision(node_id) {
            selections.push(resolution.target_symbols());
        }
        if let Some(resolution) = self.decisions()?.subscript_decision(node_id) {
            selections.push(resolution.target_symbols());
        }
        if let Some(resolution) = self.decisions()?.instantiation_decision(node_id) {
            selections.push(vec![resolution.symbol]);
        }
        if let Some(resolution) = self.resolutions()?.name_resolution(node_id) {
            selections.push(resolution.symbols().to_vec());
        }
        if let Some(resolution) = self.decisions()?.receiver_decision(node_id) {
            selections.push(vec![resolution.declaration]);
        }

        // require one authoritative resolution column
        if selections.len() > 1 {
            return Err(QueryError::conflict(format!(
                "symbol resolution columns: {node_id:?}"
            )));
        }

        Ok(selections.pop())
    }

    /// Return the symbol declarations named by one dependency item.
    pub(crate) fn dependency_declaration_symbols(
        &self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let declarations = self.dependency_declarations(item_id)?;
        let symbols = declarations
            .into_iter()
            .filter_map(|target| match target {
                dir::ReferenceTarget::Symbol(symbol_id) => Some(symbol_id),
                dir::ReferenceTarget::Namespace(_) => None,
            })
            .collect();

        Ok(symbols)
    }

    /// Return every target selected by one dependency item.
    pub(crate) fn dependency_targets(
        &self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> QueryResult<Vec<dir::ReferenceTarget>> {
        let source = item_id.into_global_any(self.module_id());
        let reference = self.resolved()?.references.get(source).ok_or_else(|| {
            QueryError::missing(format!(
                "dependency item has no resolved target: {source:?}"
            ))
        })?;

        Self::dependency_reference_targets(reference, source)
    }

    /// Return every declaration named by one dependency item.
    pub(crate) fn dependency_declarations(
        &self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> QueryResult<Vec<dir::ReferenceTarget>> {
        let source = item_id.into_global_any(self.module_id());
        let reference = self
            .resolved()?
            .references
            .declaration(source)
            .ok_or_else(|| {
                QueryError::missing(format!(
                    "dependency item has no resolved declaration: {source:?}"
                ))
            })?;

        Self::dependency_reference_targets(reference, source)
    }

    /// Return the scalar targets retained by one dependency reference.
    fn dependency_reference_targets(
        reference: &dir::Reference,
        source: dir::GlobalNodeIdAny,
    ) -> QueryResult<Vec<dir::ReferenceTarget>> {
        let targets = match reference {
            dir::Reference::Bound(symbols) => symbols
                .iter()
                .copied()
                .map(dir::ReferenceTarget::Symbol)
                .collect(),
            dir::Reference::Namespace(module) => vec![dir::ReferenceTarget::Namespace(*module)],
            dir::Reference::Projected { .. } => {
                return Err(QueryError::invalid(format!(
                    "dependency item has a projected target: {source:?}"
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
