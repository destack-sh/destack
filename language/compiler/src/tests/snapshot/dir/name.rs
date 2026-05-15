use destack_core::StringPool;
use destack_dir as dir;

/// Binding table names for DIR snapshots.
pub(super) struct BindingSnapshotName<'a> {
    /// The binding table being named.
    bindings: &'a dir::BindingTable,
    /// The string pool used by source names.
    strings: &'a StringPool,
}

impl<'a> BindingSnapshotName<'a> {
    /// Create a binding table namer.
    pub(super) fn new(bindings: &'a dir::BindingTable, strings: &'a StringPool) -> Self {
        Self { bindings, strings }
    }

    /// Return one local symbol label.
    pub(super) fn symbol(&self, symbol_id: dir::LocalSymbolId) -> String {
        let symbol = self.bindings.get_symbol(symbol_id);

        if let Some(name) = symbol.name() {
            return self.named_symbol(symbol_id, name);
        }

        Self::unnamed_symbol(symbol_id, symbol)
    }

    /// Return one scope label.
    pub(super) fn scope(&self, scope_id: dir::LocalScopeId) -> String {
        let scope = self.bindings.get_scope_by_id(scope_id);

        if let Some(owner) = scope.owner {
            return self.symbol(owner);
        }

        Self::local_scope(scope_id)
    }

    /// Return one local symbol id label.
    pub(super) fn local_symbol(symbol_id: dir::LocalSymbolId) -> String {
        format!("symbol{}", symbol_id.id)
    }

    /// Return one local scope id label.
    fn local_scope(scope_id: dir::LocalScopeId) -> String {
        format!("scope{}", scope_id.0)
    }

    /// Return one named symbol label.
    fn named_symbol(&self, symbol_id: dir::LocalSymbolId, name: dir::StringId) -> String {
        let name = self.strings.get(name).to_string();
        let duplicate_count = self.symbol_name_count(&name);
        if duplicate_count == 1 {
            return name;
        }

        let duplicate_index = self.symbol_name_index(symbol_id, &name);
        format!("{name}#{duplicate_index}")
    }

    /// Return one unnamed symbol label.
    fn unnamed_symbol(symbol_id: dir::LocalSymbolId, symbol: &dir::Symbol) -> String {
        if symbol.role == dir::SymbolRole::Namespace {
            "<module>".to_string()
        } else {
            Self::local_symbol(symbol_id)
        }
    }

    /// Count symbols with one source name.
    fn symbol_name_count(&self, name: &str) -> usize {
        self.bindings
            .symbol_ids()
            .filter(|symbol_id| {
                let symbol = self.bindings.get_symbol(*symbol_id);
                symbol
                    .name()
                    .is_some_and(|symbol_name| self.strings.get(symbol_name) == name)
            })
            .count()
    }

    /// Return the one-based duplicate index for one symbol name.
    fn symbol_name_index(&self, target_symbol_id: dir::LocalSymbolId, name: &str) -> usize {
        let mut index = 0;

        for symbol_id in self.bindings.symbol_ids() {
            let symbol = self.bindings.get_symbol(symbol_id);
            if symbol
                .name()
                .is_some_and(|symbol_name| self.strings.get(symbol_name) == name)
            {
                index += 1;
            }

            if symbol_id == target_symbol_id {
                break;
            }
        }

        index
    }
}
