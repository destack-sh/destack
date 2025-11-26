use dyst_dir::{LocalScopeMark, LocalSymbolId, Scope, Symbol, SymbolKey, SymbolTable};

use crate::TestProgram;

impl TestProgram {
    /// Resolve a symbol by name in the module's namespace scope.
    pub fn resolve_symbol_in_module(&self, module_uri: &str, name: &str) -> Option<Symbol> {
        let module_lock = self.get_module_by_uri(module_uri);
        let module = module_lock.read();
        let symbols = module.symbols.read();

        // intern the name in the program's string pool
        let name_id = self.program.strings.intern(name);
        let key = SymbolKey::Name(name_id);

        // get the namespace scope
        let namespace_scope = symbols.get_scope_by_id(module.namespace_scope);

        // resolve the symbol starting from namespace scope
        let symbol_id = self.resolve_absolute_symbol_in(
            &symbols,
            (namespace_scope, LocalScopeMark::end()),
            key,
        )?;

        Some(symbols.get_symbol(symbol_id).clone())
    }

    /// Resolve an absolute symbol by walking up scopes.
    fn resolve_absolute_symbol_in(
        &self,
        symbols: &SymbolTable,
        scope: (&Scope, LocalScopeMark),
        key: SymbolKey,
    ) -> Option<LocalSymbolId> {
        let mut scope = scope;
        loop {
            // find symbol in current scope
            if let Some(symbol_id) = scope.0.find_up_to(key, scope.1) {
                return Some(symbol_id);
            }
            // go to parent scope
            else if let Some((parent_scope_id, parent_mark)) = scope.0.parent {
                scope = (symbols.get_scope_by_id(parent_scope_id), parent_mark);
            }
            // no more scopes
            else {
                break;
            }
        }
        None
    }

    /// Resolve a relative symbol path within a scope.
    ///
    /// This takes a symbol and resolves path segments within its associated scope.
    fn resolve_relative_symbol_in(
        &self,
        symbols: &SymbolTable,
        symbol_id: LocalSymbolId,
        path_segments: &[&str],
    ) -> Option<LocalSymbolId> {
        let mut symbol = symbols.get_symbol(symbol_id);
        let mut scope = symbols.get_scope_by_symbol(symbol.id);

        // resolve path segments
        for segment in path_segments {
            let segment_id = self.program.strings.intern(*segment);
            let key = SymbolKey::Name(segment_id);
            if let Some(next_symbol_id) = scope.find(key) {
                symbol = symbols.get_symbol(next_symbol_id);
                scope = symbols.get_scope_by_symbol(symbol.id);
            } else {
                return None;
            }
        }

        Some(symbol.id)
    }

    /// Resolve a symbol path in a module.
    ///
    /// Takes a module URI and a dot-separated path like "foo.bar.baz".
    pub fn resolve_path_in_module(&self, module_uri: &str, path: &str) -> Option<Symbol> {
        let module_lock = self.get_module_by_uri(module_uri);
        let module = module_lock.read();
        let symbols = module.symbols.read();

        let segments: Vec<&str> = path.split('.').collect();
        if segments.is_empty() {
            return None;
        }

        // resolve the first segment as an absolute symbol
        let first_segment = segments[0];
        let first_name_id = self.program.strings.intern(first_segment);
        let key = SymbolKey::Name(first_name_id);

        let namespace_scope = symbols.get_scope_by_id(module.namespace_scope);
        let symbol_id = self.resolve_absolute_symbol_in(
            &symbols,
            (namespace_scope, LocalScopeMark::end()),
            key,
        )?;

        // resolve remaining segments as relative
        if segments.len() > 1 {
            let remaining_segments = &segments[1..];
            let resolved_id =
                self.resolve_relative_symbol_in(&symbols, symbol_id, remaining_segments)?;
            Some(symbols.get_symbol(resolved_id).clone())
        } else {
            Some(symbols.get_symbol(symbol_id).clone())
        }
    }
}

