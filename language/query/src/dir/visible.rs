use destack_dir::{
    BindingTable, LocalScopeId, LocalScopeMark, LocalSymbolId, StaticKey, Symbol, SymbolSpace,
};

use super::matches_symbol_space_filter;

/// Information about a visible symbol.
#[derive(Debug, Clone)]
pub(crate) struct VisibleSymbol<'a> {
    /// The symbol id.
    pub id: LocalSymbolId,
    /// The symbol.
    pub symbol: &'a Symbol,
    /// The key (name) of the symbol.
    pub key: StaticKey,
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
/// * `space_filter` - Optional filter for symbol space.
pub(crate) fn visible_symbols<'a>(
    symbols: &'a BindingTable<'a>,
    scope_id: LocalScopeId,
    mark: LocalScopeMark,
    space_filter: Option<SymbolSpace>,
) -> impl Iterator<Item = VisibleSymbol<'a>> + 'a {
    VisibleSymbolIterator {
        symbols,
        current_scope_id: Some(scope_id),
        current_mark: mark,
        space_filter,
        seen_index: 0,
    }
}

/// Iterator for walking visible symbols up the scope chain.
struct VisibleSymbolIterator<'a> {
    symbols: &'a BindingTable<'a>,
    current_scope_id: Option<LocalScopeId>,
    current_mark: LocalScopeMark,
    space_filter: Option<SymbolSpace>,
    seen_index: usize,
}

impl<'a> Iterator for VisibleSymbolIterator<'a> {
    type Item = VisibleSymbol<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let scope_id = self.current_scope_id?;
            let scope = self.symbols.get_scope_by_id(scope_id);

            // get named bindings up to the mark
            let limit = self.current_mark.0 as usize;
            let bindings = &scope.bindings[..limit.min(scope.bindings.len())];

            // try to find next valid symbol in current scope
            while self.seen_index < bindings.len() {
                let binding = bindings[self.seen_index];
                self.seen_index += 1;
                let Some(key) = binding.key else {
                    continue;
                };
                let symbol_id = binding.symbol;

                let symbol = self.symbols.get_symbol(symbol_id);

                // filter by space if requested
                if let Some(space) = self.space_filter {
                    let matches = matches_symbol_space_filter(symbol.form, Some(space));
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
