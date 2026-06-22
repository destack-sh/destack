use destack_dir as dir;
use destack_qir::SymbolUse;

use super::symbol_matches_use;

/// Information about a visible symbol.
#[derive(Debug, Clone)]
pub(crate) struct VisibleSymbol<'a> {
    /// The symbol id.
    pub id: dir::LocalSymbolId,
    /// The symbol.
    pub symbol: &'a dir::Symbol,
    /// The key (name) of the symbol.
    pub key: dir::StaticKey,
}

/// Iterate visible symbols starting from a scope, walking up the scope chain.
///
/// This is the primary helper for collecting symbols visible at a given position.
/// Used for completions, diagnostics, and other scope-aware queries.
///
/// # Arguments
/// * `symbols` - The symbol table
/// * `scope_id` - Starting scope id
/// * `mark` - Scope mark (position within the scope)
/// * `use_filter` - Optional filter for symbol use.
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
        seen_index: 0,
    }
}

/// Iterator for walking visible symbols up the scope chain.
struct VisibleSymbolIterator<'a> {
    symbols: &'a dir::BindingTable<'a>,
    current_scope_id: Option<dir::LocalScopeId>,
    current_mark: dir::LocalScopeMark,
    use_filter: Option<SymbolUse>,
    seen_index: usize,
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
            while self.seen_index < bindings.len() {
                let index = self.seen_index;
                let binding = bindings[self.seen_index];
                self.seen_index += 1;
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
                    let matches = symbol_matches_use(symbol.kind, Some(symbol_use));
                    if !matches {
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
                self.seen_index = 0;
            } else {
                self.current_scope_id = None;
            }
        }
    }
}
