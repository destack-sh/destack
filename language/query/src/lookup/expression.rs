use tspp_dir as dir;

use crate::{ModuleQueryContext, QueryError, QueryResult};

impl ModuleQueryContext<'_> {
    /// Return every symbol target from one recorded use-site resolution.
    pub(crate) fn symbol_targets(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> QueryResult<Option<Vec<dir::GlobalSymbolId>>> {
        // select symbols from the single checked decision
        let decision = self.decisions()?.decision(node_id);
        let targets = match decision {
            Some(dir::Decision::Member(selection)) => Some(selection.target_symbols()),
            Some(dir::Decision::Subscript(selection)) => Some(selection.target_symbols()),
            Some(dir::Decision::Function(selection)) => {
                let targets = selection
                    .arms()
                    .iter()
                    .filter_map(|value| value.target.symbol())
                    .collect::<Vec<_>>();

                (!targets.is_empty()).then_some(targets)
            }
            Some(dir::Decision::Receiver(selection)) => Some(vec![selection.declaration]),
            Some(dir::Decision::Transfer(_)) => self
                .transfer_label_symbol(node_id)?
                .map(|symbol| vec![symbol]),
            _ => None,
        };

        // a selected function determines which declaration its name denotes
        if matches!(decision, Some(dir::Decision::Function(_))) && targets.is_some() {
            return Ok(targets);
        }

        // other recorded names must identify a single source of symbol targets
        let name = self
            .resolutions()?
            .name_resolution(node_id)
            .filter(|resolution| resolution.denoted_type().is_none());
        match (targets, name) {
            (Some(_), Some(_)) => Err(QueryError::conflict(format!(
                "symbol resolution columns: {node_id:?}"
            ))),
            (Some(targets), None) => Ok(Some(targets)),
            (None, Some(name)) => Ok(Some(name.symbols().to_vec())),
            (None, None) => Ok(None),
        }
    }

    /// Return the label symbol explicitly named by one control transfer.
    pub(crate) fn transfer_label_symbol(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> QueryResult<Option<dir::GlobalSymbolId>> {
        // require a checked control transfer
        let Some(target) = self.decisions()?.transfer_decision(node_id) else {
            return Ok(None);
        };

        // require an explicitly authored label reference
        let expression = node_id
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| {
                QueryError::invalid(format!(
                    "control transfer source is not an expression: {node_id:?}"
                ))
            })?;
        if self.view()?.get(expression).transfer_label().is_none() {
            return Ok(None);
        }

        // read the binding introduced by the selected labeled target
        let symbol = self
            .bindings()?
            .declaration_symbol(target.into_any())
            .ok_or_else(|| QueryError::missing(format!("control label binding: {target:?}")))?;
        let symbol = symbol.into_global(target.module_id);

        Ok(Some(symbol))
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
            dir::Reference::Namespace { module, .. } => {
                vec![dir::ReferenceTarget::Namespace(*module)]
            }
            dir::Reference::Projected { .. } => {
                return Err(QueryError::invalid(format!(
                    "dependency item has a projected target: {source:?}"
                )));
            }
            dir::Reference::Ambiguous(targets) => targets.to_vec(),
            dir::Reference::TypeLiteral(_) | dir::Reference::Missing => Vec::new(),
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
