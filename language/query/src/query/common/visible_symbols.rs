use destack_dir::{
    LocalScopeId, LocalScopeMark, LocalSymbolId, StaticKey, Symbol, SymbolSpace, SymbolTable,
};

use super::resolve::matches_symbol_space_filter;

/// Information about a visible symbol.
#[derive(Debug, Clone)]
pub struct VisibleSymbol<'a> {
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
/// * `space_filter` - Optional filter for symbol space (Value, Type, or TypeValue)
pub fn visible_symbols<'a>(
    symbols: &'a SymbolTable,
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

/// Iterate visible symbols starting from a scope without a mark.
///
/// Uses the full scope (no mark restriction).
pub fn visible_symbols_full<'a>(
    symbols: &'a SymbolTable,
    scope_id: LocalScopeId,
    space_filter: Option<SymbolSpace>,
) -> impl Iterator<Item = VisibleSymbol<'a>> + 'a {
    visible_symbols(symbols, scope_id, LocalScopeMark::end(), space_filter)
}

/// Iterator for walking visible symbols up the scope chain.
struct VisibleSymbolIterator<'a> {
    symbols: &'a SymbolTable,
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

            // get symbols up to the mark
            let limit = self.current_mark.0 as usize;
            let named_symbols = &scope.named_symbols[..limit.min(scope.named_symbols.len())];

            // try to find next valid symbol in current scope
            while self.seen_index < named_symbols.len() {
                let (key, symbol_id) = named_symbols[self.seen_index];
                self.seen_index += 1;

                let symbol = self.symbols.get_symbol(symbol_id);

                // skip inactive symbols
                if !symbol.is_active {
                    continue;
                }

                // filter by space if requested
                if let Some(space) = self.space_filter {
                    let matches = matches_symbol_space_filter(symbol.ty, symbol.space, Some(space));
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
            if let Some((parent_id, parent_mark)) = scope.parent {
                self.current_scope_id = Some(parent_id);
                self.current_mark = parent_mark;
                self.seen_index = 0;
            } else {
                self.current_scope_id = None;
            }
        }
    }
}

/// Collect all visible symbol names for duplicate checking.
pub fn collect_visible_names(
    symbols: &SymbolTable,
    scope_id: LocalScopeId,
    mark: LocalScopeMark,
    space_filter: Option<SymbolSpace>,
) -> Vec<StaticKey> {
    visible_symbols(symbols, scope_id, mark, space_filter)
        .map(|vs| vs.key)
        .collect()
}
