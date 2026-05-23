use destack_core::StringPool;
use destack_dir as dir;

/// Binding table names for DIR snapshots.
pub(super) struct BindingSnapshotName<'a> {
    /// The binding table being named.
    bindings: &'a dir::BindingTable<'a>,
    /// The DIR tree that owns the bindings when available.
    tree: Option<&'a dir::Tree>,
    /// The string pool used by source names.
    strings: &'a StringPool,
}

impl<'a> BindingSnapshotName<'a> {
    /// Create a binding table namer.
    pub(super) fn new(
        bindings: &'a dir::BindingTable<'a>,
        tree: Option<&'a dir::Tree>,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            bindings,
            tree,
            strings,
        }
    }

    /// Return one local symbol label.
    pub(super) fn symbol(&self, symbol_id: dir::LocalSymbolId) -> String {
        let symbol = self.bindings.get_symbol(symbol_id);

        if let Some(name) = symbol.name() {
            return self.named_symbol(symbol_id, name);
        }

        Self::unnamed_symbol(symbol_id, symbol)
    }

    /// Return one semantic symbol path label.
    pub(super) fn symbol_path(&self, symbol_id: dir::LocalSymbolId) -> String {
        let path = self.symbol_path_base(symbol_id);
        let duplicate_count = self.symbol_path_count(&path);
        if duplicate_count == 1 {
            return path;
        }

        let duplicate_index = self.symbol_path_index(symbol_id, &path);
        format!("{path}#{duplicate_index}")
    }

    /// Return one semantic symbol path without duplicate suffixes.
    fn symbol_path_base(&self, symbol_id: dir::LocalSymbolId) -> String {
        let symbol = self.bindings.get_symbol(symbol_id);
        let label = self.symbol_base_label(symbol_id, symbol);
        if !self.symbol_should_qualify(symbol) {
            return label;
        }

        let Some(owner) = self.symbol_scope_owner(symbol) else {
            return label;
        };

        let owner_symbol = self.bindings.get_symbol(owner);
        if owner_symbol.role == dir::SymbolRole::Namespace && owner_symbol.name().is_none() {
            return label;
        }

        let owner = self.symbol_path_base(owner);

        format!("{owner}.{label}")
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

    /// Return one symbol label without duplicate suffixes.
    fn symbol_base_label(&self, symbol_id: dir::LocalSymbolId, symbol: &dir::Symbol) -> String {
        if let Some(name) = symbol.name() {
            return self.strings.get(name).to_string();
        }
        if let Some(label) = self.member_slot_label(symbol) {
            return label.to_string();
        }

        Self::unnamed_symbol(symbol_id, symbol)
    }

    /// Return the label for an anonymous role member symbol.
    fn member_slot_label(&self, symbol: &dir::Symbol) -> Option<&'static str> {
        let tree = self.tree?;
        let declaration = symbol.declaration?;
        if declaration.local_id.ty != dir::NodeType::Member {
            return None;
        }

        let member = tree.get(dir::LocalNodeId::<dir::Member>::new(
            declaration.local_id.id,
        ));
        match member.slot()? {
            dir::MemberSlot::Constructor => Some("constructor"),
            dir::MemberSlot::New => Some("new"),
            dir::MemberSlot::Call => Some("<call>"),
            dir::MemberSlot::Key(_) => None,
        }
    }

    /// Return one unnamed symbol label.
    fn unnamed_symbol(symbol_id: dir::LocalSymbolId, symbol: &dir::Symbol) -> String {
        if symbol.role == dir::SymbolRole::Namespace {
            "<module>".to_string()
        } else {
            Self::local_symbol(symbol_id)
        }
    }

    /// Return whether one symbol should be path-qualified.
    fn symbol_should_qualify(&self, symbol: &dir::Symbol) -> bool {
        symbol.role == dir::SymbolRole::Item
            || symbol.role == dir::SymbolRole::Namespace
            || symbol.form == dir::SymbolForm::TypeAlias
    }

    /// Return the owner symbol for the symbol scope.
    fn symbol_scope_owner(&self, symbol: &dir::Symbol) -> Option<dir::LocalSymbolId> {
        let scope = self.bindings.get_scope_by_id(symbol.scope.id);

        scope.owner
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

    /// Count symbols with one semantic path.
    fn symbol_path_count(&self, path: &str) -> usize {
        self.bindings
            .symbol_ids()
            .filter(|symbol_id| self.symbol_path_base(*symbol_id) == path)
            .count()
    }

    /// Return the one-based duplicate index for one semantic symbol path.
    fn symbol_path_index(&self, target_symbol_id: dir::LocalSymbolId, path: &str) -> usize {
        let mut index = 0;

        for symbol_id in self.bindings.symbol_ids() {
            if self.symbol_path_base(symbol_id) == path {
                index += 1;
            }

            if symbol_id == target_symbol_id {
                break;
            }
        }

        index
    }
}
