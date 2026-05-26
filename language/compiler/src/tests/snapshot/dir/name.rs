use std::collections::BTreeMap;

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
    /// Source symbol labels keyed by local symbol.
    symbol_labels: BTreeMap<dir::LocalSymbolId, String>,
    /// Semantic symbol path labels keyed by local symbol.
    symbol_path_labels: BTreeMap<dir::LocalSymbolId, String>,
}

impl<'a> BindingSnapshotName<'a> {
    /// Create a binding table namer.
    pub(super) fn new(
        bindings: &'a dir::BindingTable<'a>,
        tree: Option<&'a dir::Tree>,
        strings: &'a StringPool,
    ) -> Self {
        let mut names = Self {
            bindings,
            tree,
            strings,
            symbol_labels: BTreeMap::new(),
            symbol_path_labels: BTreeMap::new(),
        };

        // build labels once for the whole table
        names.symbol_labels = names.build_symbol_labels();
        names.symbol_path_labels = names.build_symbol_path_labels();

        names
    }

    /// Return one local symbol label.
    pub(super) fn symbol(&self, symbol_id: dir::LocalSymbolId) -> String {
        self.symbol_labels
            .get(&symbol_id)
            .cloned()
            .unwrap_or_else(|| panic!("dir snapshot missing symbol label for {symbol_id:?}"))
    }

    /// Return one semantic symbol path label.
    pub(super) fn symbol_path(&self, symbol_id: dir::LocalSymbolId) -> String {
        self.symbol_path_labels
            .get(&symbol_id)
            .cloned()
            .unwrap_or_else(|| panic!("dir snapshot missing symbol path label for {symbol_id:?}"))
    }

    /// Return all semantic symbol path labels.
    pub(super) fn symbol_path_labels(&self) -> BTreeMap<dir::LocalSymbolId, String> {
        self.symbol_path_labels.clone()
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

    /// Build all source symbol labels for this table.
    fn build_symbol_labels(&self) -> BTreeMap<dir::LocalSymbolId, String> {
        let symbol_ids = self.bindings.symbol_ids().collect::<Vec<_>>();
        let mut counts = BTreeMap::<String, usize>::new();

        // count duplicate source names
        for symbol_id in &symbol_ids {
            let symbol = self.bindings.get_symbol(*symbol_id);
            if let Some(name) = symbol.name() {
                let name = self.strings.get(name).to_string();
                *counts.entry(name).or_insert(0) += 1;
            }
        }

        let mut indexes = BTreeMap::<String, usize>::new();
        let mut labels = BTreeMap::new();

        // label symbols in stable binding order
        for symbol_id in symbol_ids {
            let symbol = self.bindings.get_symbol(symbol_id);
            let label = if let Some(name) = symbol.name() {
                self.duplicate_label(name, &counts, &mut indexes)
            } else {
                Self::unnamed_symbol(symbol_id, symbol)
            };

            labels.insert(symbol_id, label);
        }

        labels
    }

    /// Build all semantic symbol path labels for this table.
    fn build_symbol_path_labels(&self) -> BTreeMap<dir::LocalSymbolId, String> {
        let symbol_ids = self.bindings.symbol_ids().collect::<Vec<_>>();
        let mut bases = BTreeMap::new();

        // build recursive path bases once
        for symbol_id in &symbol_ids {
            self.symbol_path_base(*symbol_id, &mut bases);
        }

        let mut counts = BTreeMap::<String, usize>::new();
        for path in bases.values() {
            *counts.entry(path.clone()).or_insert(0) += 1;
        }

        let mut indexes = BTreeMap::<String, usize>::new();
        let mut labels = BTreeMap::new();

        // suffix duplicate paths in stable binding order
        for symbol_id in symbol_ids {
            let path = bases.get(&symbol_id).cloned().unwrap_or_else(|| {
                panic!("dir snapshot missing symbol path base for {symbol_id:?}")
            });
            let count = counts.get(&path).copied().unwrap_or(0);
            if count == 1 {
                labels.insert(symbol_id, path);
            } else {
                let index = indexes.entry(path.clone()).or_insert(0);
                *index += 1;
                labels.insert(symbol_id, format!("{path}#{index}"));
            }
        }

        labels
    }

    /// Return one duplicate-aware source name label.
    fn duplicate_label(
        &self,
        name: dir::StringId,
        counts: &BTreeMap<String, usize>,
        indexes: &mut BTreeMap<String, usize>,
    ) -> String {
        let name = self.strings.get(name).to_string();
        let count = counts.get(&name).copied().unwrap_or(0);
        if count == 1 {
            return name;
        }

        let index = indexes.entry(name.clone()).or_insert(0);
        *index += 1;

        format!("{name}#{index}")
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
            || symbol.kind == dir::SymbolKind::TypeAlias
    }

    /// Return the owner symbol for the symbol scope.
    fn symbol_scope_owner(&self, symbol: &dir::Symbol) -> Option<dir::LocalSymbolId> {
        let scope = self.bindings.get_scope_by_id(symbol.scope.id);

        scope.owner
    }

    /// Return one semantic symbol path without duplicate suffixes.
    fn symbol_path_base(
        &self,
        symbol_id: dir::LocalSymbolId,
        bases: &mut BTreeMap<dir::LocalSymbolId, String>,
    ) -> String {
        if let Some(path) = bases.get(&symbol_id) {
            return path.clone();
        }

        let symbol = self.bindings.get_symbol(symbol_id);
        let label = self.symbol_base_label(symbol_id, symbol);
        if !self.symbol_should_qualify(symbol) {
            bases.insert(symbol_id, label.clone());

            return label;
        }

        let Some(owner) = self.symbol_scope_owner(symbol) else {
            bases.insert(symbol_id, label.clone());

            return label;
        };

        let owner_symbol = self.bindings.get_symbol(owner);
        if owner_symbol.role == dir::SymbolRole::Namespace && owner_symbol.name().is_none() {
            bases.insert(symbol_id, label.clone());

            return label;
        }

        let owner = self.symbol_path_base(owner, bases);
        let path = format!("{owner}.{label}");
        bases.insert(symbol_id, path.clone());

        path
    }
}
