use destack_dir as dir;

use crate::SymbolUse;

/// One symbol visible from a scope walk.
#[derive(Debug, Clone)]
pub(crate) struct VisibleSymbol<'a> {
    /// The symbol id.
    pub id: dir::LocalSymbolId,
    /// The symbol.
    pub symbol: &'a dir::Symbol,
    /// The key (name) of the symbol.
    pub key: dir::StaticKey,
}

/// Iterate visible symbols from a scope, walking up the parent scope chain.
pub(crate) fn visible_symbols<'a>(
    symbols: &'a dir::BindingTable<'a>,
    scope_id: dir::LocalScopeId,
    mark: dir::LocalScopeMark,
    use_filter: Option<SymbolUse>,
) -> impl Iterator<Item = VisibleSymbol<'a>> + 'a {
    VisibleSymbolIterator {
        symbols,
        current_scope_id: Some(scope_id),
        current_mark: mark,
        use_filter,
        binding_index: 0,
    }
}

/// Iterator for walking visible symbols up the scope chain.
struct VisibleSymbolIterator<'a> {
    /// The checked DIR binding table.
    symbols: &'a dir::BindingTable<'a>,
    /// The scope currently being scanned.
    current_scope_id: Option<dir::LocalScopeId>,
    /// The visible binding boundary for the current scope.
    current_mark: dir::LocalScopeMark,
    /// The optional type or value use filter.
    use_filter: Option<SymbolUse>,
    /// The next binding index to scan in the current scope.
    binding_index: usize,
}

impl<'a> Iterator for VisibleSymbolIterator<'a> {
    type Item = VisibleSymbol<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let scope_id = self.current_scope_id?;
            let scope = self.symbols.get_scope_by_id(scope_id);

            // scan the current scope bindings
            let limit = self.current_mark.0 as usize;
            let bindings = &scope.bindings;

            // try to find next valid symbol in current scope
            while self.binding_index < bindings.len() {
                let index = self.binding_index;
                let binding = bindings[self.binding_index];
                self.binding_index += 1;
                let Some(key) = binding.key else {
                    continue;
                };
                let symbol_id = binding.symbol;

                let symbol = self.symbols.get_symbol(symbol_id);

                // skip forward bindings after the cursor
                if symbol.visibility == dir::SymbolVisibility::Forward && index >= limit {
                    continue;
                }

                // filter by use if requested
                if let Some(symbol_use) = self.use_filter {
                    if !symbol_use.accepts_symbol_kind(symbol.kind) {
                        continue;
                    }
                }

                return Some(VisibleSymbol {
                    id: symbol_id,
                    symbol,
                    key,
                });
            }

            // move to parent scope
            if let Some(parent) = scope.parent {
                self.current_scope_id = Some(parent.id);
                self.current_mark = parent.mark;
                self.binding_index = 0;
            } else {
                self.current_scope_id = None;
            }
        }
    }
}
