use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Iterate checked identifier references to one binding.
    pub(crate) fn binding_references(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> impl Iterator<Item = dir::LocalNodeId<dir::Expression>> + '_ {
        self.resolutions
            .name_entries()
            .filter_map(move |(node, resolution)| {
                if resolution.single_symbol() != Some(symbol) {
                    return None;
                }

                let expression = node.local_id.try_into_typed::<dir::Expression>().ok()?;
                matches!(
                    self.view().get(expression),
                    dir::Expression::Identifier { .. }
                )
                .then_some(expression)
            })
    }

    /// Return the only checked symbol declared within one node subtree.
    pub(crate) fn sole_declared_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let mut symbols = self.symbols_declared_within(node);
        let symbol = symbols.next()?;

        symbols.next().is_none().then_some(symbol)
    }

    /// Iterate the checked symbols declared within one node subtree.
    pub(crate) fn symbols_declared_within(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> impl Iterator<Item = dir::GlobalSymbolId> + '_ {
        let view = self.view();

        self.bindings
            .declaration_symbols()
            .filter(move |(declaration, _)| view.is_inside(declaration.local_id, node))
            .map(|(_, symbol)| symbol.into_global(self.id))
    }

    /// Return the recorded uses of one binding within a node subtree.
    pub(crate) fn binding_uses_within(
        &self,
        symbol: dir::GlobalSymbolId,
        node: dir::LocalNodeIdAny,
        occurrences: &[dir::BindingOccurrence],
    ) -> dir::BindingUse {
        let view = self.view();
        let mut uses = dir::BindingUse::default();

        // merge occurrences within the selected node
        for occurrence in occurrences {
            if occurrence.symbol == symbol && view.is_inside(occurrence.node, node) {
                uses |= occurrence.uses;
            }
        }

        uses
    }

    /// Return the recorded uses of one binding outside selected node subtrees.
    pub(crate) fn binding_uses_outside(
        &self,
        symbol: dir::GlobalSymbolId,
        nodes: &[dir::LocalNodeIdAny],
        occurrences: &[dir::BindingOccurrence],
    ) -> dir::BindingUse {
        let view = self.view();
        let mut uses = dir::BindingUse::default();

        // merge occurrences outside every selected node
        for occurrence in occurrences {
            if occurrence.symbol != symbol {
                continue;
            }
            if nodes
                .iter()
                .all(|node| !view.is_inside(occurrence.node, *node))
            {
                uses |= occurrence.uses;
            }
        }

        uses
    }

    /// Return the declaration node that introduced one checked symbol.
    pub fn symbol_declaration(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<dir::GlobalNodeIdAny, ProviderError> {
        if symbol.module_id != self.id {
            return Err(ProviderError::internal(format!(
                "symbol {symbol:?} belongs to another module"
            )));
        }

        // read the checked symbol declaration
        let binding = self
            .bindings
            .get_symbol_maybe(symbol.local_id)
            .ok_or_else(|| ProviderError::internal(format!("missing checked symbol {symbol:?}")))?;
        let declaration = binding.declaration.ok_or_else(|| {
            ProviderError::internal(format!("checked symbol {symbol:?} has no declaration"))
        })?;

        Ok(declaration)
    }

    /// Return the symbol introduced by one checked declaration node.
    pub fn declaration_symbol<T: dir::Node>(
        &self,
        node: dir::LocalNodeId<T>,
    ) -> Result<dir::GlobalSymbolId, ProviderError> {
        // read the checked declaration binding
        let declaration = node.into_global_any(self.id);
        let symbol = self
            .bindings
            .declaration_symbol(declaration)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "checked declaration {declaration:?} introduces no symbol"
                ))
            })?;

        Ok(symbol.into_global(self.id))
    }

    /// Return the single declaration symbol selected directly by one checked expression.
    pub fn selected_symbol(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
        // select the checked resolution for supported expression forms
        let view = self.view();
        let global = node.into_global_any(self.id);
        let symbol = match view.get(node) {
            dir::Expression::Identifier { .. } => {
                let resolution = self.resolutions.name_resolution(global).ok_or_else(|| {
                    ProviderError::internal(format!(
                        "checked identifier expression {} in module {:?} has no name resolution",
                        node.id, self.id
                    ))
                })?;
                let [symbol] = resolution.symbols() else {
                    return Ok(None);
                };

                *symbol
            }
            dir::Expression::Member { .. } => {
                let Some(resolution) = self.member_decision(node)? else {
                    return Ok(None);
                };
                let dir::OperationResolution::One(access) = resolution else {
                    return Ok(None);
                };
                let Some(symbol) = access.target.symbol() else {
                    return Ok(None);
                };

                symbol
            }
            _ => return Ok(None),
        };

        Ok(Some(symbol))
    }
}
