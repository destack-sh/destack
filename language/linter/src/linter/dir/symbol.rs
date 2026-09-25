use tspp_dir as dir;
use tspp_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return an unused binding name derived from one preferred name.
    pub(crate) fn fresh_binding_name(&self, preferred: &str) -> String {
        let mut candidate = preferred.to_string();
        let mut suffix = 2;

        // advance until no binding uses the candidate
        while self.bindings.symbols().any(|symbol| {
            symbol
                .name()
                .is_some_and(|name| self.dir.strings.get(name) == candidate)
        }) {
            candidate = format!("{preferred}{suffix}");
            suffix += 1;
        }

        candidate
    }

    /// Return whether declarations in nested subtrees shadow bindings in enclosing subtrees.
    pub(crate) fn shadows_bindings(
        &self,
        nested: impl IntoIterator<Item = dir::LocalNodeIdAny>,
        enclosing: impl IntoIterator<Item = dir::LocalNodeIdAny>,
    ) -> bool {
        // collect the names introduced by the enclosing subtrees
        let enclosing_names = enclosing
            .into_iter()
            .flat_map(|node| self.symbols_declared_within(node))
            .filter_map(|symbol| self.bindings.get_symbol(symbol.local_id).name())
            .collect::<Vec<_>>();

        // find a nested declaration with one of the same names
        nested
            .into_iter()
            .flat_map(|node| self.symbols_declared_within(node))
            .filter_map(|symbol| self.bindings.get_symbol(symbol.local_id).name())
            .any(|name| enclosing_names.contains(&name))
    }

    /// Iterate name references to one binding.
    pub(crate) fn symbol_references(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> impl Iterator<Item = dir::LocalNodeIdAny> + '_ {
        self.resolutions
            .name_entries()
            .filter_map(move |(node, resolution)| {
                (resolution.single_symbol() == Some(symbol)).then_some(node.local_id)
            })
    }

    /// Iterate identifier references to one binding.
    pub(crate) fn binding_references(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> impl Iterator<Item = dir::LocalNodeId<dir::Expression>> + '_ {
        self.symbol_references(symbol).filter_map(|node| {
            let expression = node.try_into_typed::<dir::Expression>().ok()?;
            matches!(
                self.view().get(expression),
                dir::Expression::Identifier { .. }
            )
            .then_some(expression)
        })
    }

    /// Return the only symbol declared within one node subtree.
    pub(crate) fn sole_declared_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let mut symbols = self.symbols_declared_within(node);
        let symbol = symbols.next()?;

        symbols.next().is_none().then_some(symbol)
    }

    /// Iterate the symbols declared within one node subtree.
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

    /// Return the recorded uses of bindings declared within one node subtree.
    pub(crate) fn declared_binding_uses(
        &self,
        node: dir::LocalNodeIdAny,
        occurrences: &[dir::BindingOccurrence],
    ) -> dir::BindingUse {
        let mut uses = dir::BindingUse::default();

        // merge occurrences of every binding introduced within the node
        for symbol in self.symbols_declared_within(node) {
            for occurrence in occurrences {
                if occurrence.symbol == symbol {
                    uses |= occurrence.uses;
                }
            }
        }

        uses
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

    /// Return the recorded uses of one binding after a source node.
    pub(crate) fn binding_uses_after(
        &self,
        symbol: dir::GlobalSymbolId,
        node: dir::LocalNodeIdAny,
        occurrences: &[dir::BindingOccurrence],
    ) -> Result<dir::BindingUse, ProviderError> {
        let extent = self.source_extent(node)?;
        let mut uses = dir::BindingUse::default();

        // merge later occurrences in the same authored file
        for occurrence in occurrences {
            if occurrence.symbol != symbol {
                continue;
            }
            let occurrence_extent = self.source_extent(occurrence.node)?;
            if occurrence_extent.file == extent.file && occurrence_extent.start >= extent.end {
                uses |= occurrence.uses;
            }
        }

        Ok(uses)
    }

    /// Return the declaration node that introduced one symbol.
    pub fn symbol_declaration(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<dir::GlobalNodeIdAny, ProviderError> {
        if symbol.module_id != self.id {
            return Err(ProviderError::internal(format!(
                "symbol {symbol:?} belongs to another module"
            )));
        }

        // read the symbol declaration
        let binding = self
            .bindings
            .get_symbol_maybe(symbol.local_id)
            .ok_or_else(|| ProviderError::internal(format!("missing symbol {symbol:?}")))?;
        let declaration = binding.declaration.ok_or_else(|| {
            ProviderError::internal(format!("symbol {symbol:?} has no declaration"))
        })?;

        Ok(declaration)
    }

    /// Return the symbol introduced by one declaration node.
    pub fn declaration_symbol<T: dir::Node>(
        &self,
        node: dir::LocalNodeId<T>,
    ) -> Result<dir::GlobalSymbolId, ProviderError> {
        // read the declaration binding
        let declaration = node.into_global_any(self.id);
        let symbol = self
            .bindings
            .declaration_symbol(declaration)
            .ok_or_else(|| {
                ProviderError::internal(format!("declaration {declaration:?} introduces no symbol"))
            })?;

        Ok(symbol.into_global(self.id))
    }

    /// Return the single declaration symbol selected directly by one expression.
    pub fn selected_symbol(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
        // select the resolution for supported expression forms
        let view = self.view();
        let global = node.into_global_any(self.id);
        let symbol = match view.get(node) {
            dir::Expression::Identifier { .. } => {
                let resolution = self.resolutions.name_resolution(global).ok_or_else(|| {
                    ProviderError::internal(format!(
                        "identifier expression {} in module {:?} has no name resolution",
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
