use std::sync::Arc;

use destack_artifact::{DirExpanded, DirParsed};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use super::CheckState;

/// Committed tables loaded for one out-of-component dependency module.
pub(in crate::check) struct CheckDependencyState {
    /// The parsed dependency module.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The expanded dependency module.
    pub(in crate::check) expanded: Arc<DirExpanded>,
    /// The committed binding table.
    pub(in crate::check) bindings: dir::BindingTable<'static>,
    /// The committed type table.
    pub(in crate::check) types: dir::TypeTable<'static>,
    /// The committed static table.
    pub(in crate::check) statics: dir::StaticTable<'static>,
    /// The committed generic table.
    pub(in crate::check) generics: dir::GenericTable<'static>,
    /// The committed nominal table.
    pub(in crate::check) nominals: dir::NominalTable<'static>,
    /// The committed extension table.
    pub(in crate::check) extensions: dir::ExtensionTable<'static>,
}

impl CheckDependencyState {
    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }

    /// Return named member symbols declared under one dependency owner.
    pub(in crate::check) fn named_member_symbols(
        &self,
        owner: dir::GlobalSymbolId,
    ) -> Vec<(dir::StaticKey, dir::GlobalSymbolId)> {
        let Some(scope) = self.bindings.scope_for_owner(owner.local_id) else {
            return Vec::new();
        };
        let scope = self.bindings.get_scope(scope);

        scope
            .named_symbols()
            .map(|(key, symbol)| (key, symbol.into_global(owner.module_id)))
            .collect()
    }

    /// Return member symbols declared under one dependency owner and key.
    pub(in crate::check) fn member_symbols(
        &self,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        match self.bindings.lookup_key_member(owner.local_id, key) {
            dir::SymbolLookup::Missing => SmallVec::new(),
            dir::SymbolLookup::Found(symbol) => {
                let mut symbols = SmallVec::new();
                symbols.push(symbol.into_global(owner.module_id));

                symbols
            }
            dir::SymbolLookup::Ambiguous(symbols) => symbols
                .into_iter()
                .map(|symbol| symbol.into_global(owner.module_id))
                .collect(),
        }
    }
}

impl CheckState<'_> {
    /// Return loaded state for one dependency module.
    pub(in crate::check) fn dependency(&self, module: ModuleId) -> &CheckDependencyState {
        self.dependencies
            .get(&module)
            .unwrap_or_else(|| unreachable!("dependency {module:?} was not loaded"))
    }
}
