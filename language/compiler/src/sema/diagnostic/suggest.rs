use destack_core::{NameMatch, find_best_match};
use destack_dir as dir;
use destack_source::{DiagnosticSuggestion, ModuleId};

use crate::sema::CheckState;
use crate::{DiagnosticAnchor, diagnostic_suggestion_distance, rename_suggestion};

impl CheckState<'_> {
    /// Return a human readable path label.
    pub(in crate::sema) fn path_label(&self, path: &dir::Path) -> String {
        let mut label = String::new();

        // join path segments with dot notation
        for (index, segment) in path.segments.iter().enumerate() {
            if index > 0 {
                label.push('.');
            }

            label.push_str(self.strings().get(*segment));
        }

        label
    }

    /// Return one imported declaration matching an unresolved name.
    pub(in crate::sema) fn imported_declaration(
        &self,
        module: ModuleId,
        path: &dir::Path,
    ) -> Option<dir::GlobalSymbolId> {
        let [name] = path.segments.as_slice() else {
            return None;
        };
        let name = self.strings().get(*name);

        // collect the modules this file already imports from
        let mut imported = Vec::new();
        for target in self.module_maybe(module)?.resolved.imports.targets() {
            let target = target.module();
            if target != module && !imported.contains(&target) {
                imported.push(target);
            }
        }

        // find an exact declaration in one module scope
        let declaration = |bindings: &dir::BindingTable<'_>| {
            let scope = bindings.module_scope();
            bindings
                .get_scope(scope)
                .named_symbols_up_to(scope.mark)
                .find_map(|(key, symbol)| {
                    matches!(key, dir::StaticKey::Name(key) if self.strings().get(key) == name)
                        .then_some(symbol)
                })
        };

        // select the first matching imported declaration
        for imported in imported {
            let external = self.external_module(imported);
            let symbol = declaration(&external.bindings);
            if let Some(symbol) = symbol {
                return Some(symbol.into_global(imported));
            }
        }

        None
    }

    /// Return the closest visible name for one unresolved single-segment path.
    pub(in crate::sema) fn closest_reference_name(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> Option<NameMatch<String>> {
        let [name] = path.segments.as_slice() else {
            return None;
        };
        let name = self.strings().get(*name).to_string();

        let bindings = self.module(module).binding_table();
        let scope = bindings.scope_at(&self.module(module).view(), source);
        let mut candidates = Vec::new();

        // collect lexical names visible at the source node
        self.collect_reference_names(&bindings, scope, &mut candidates);

        // collect profile-provided globals visible to unresolved references
        for key in self
            .module(module)
            .resolved
            .imports
            .global_resolution_by_key
            .keys()
        {
            if let Some(candidate) = self.reference_key_text(key) {
                candidates.push(candidate);
            }
        }

        find_best_match(&name, candidates, diagnostic_suggestion_distance(&name))
    }

    /// Return the rename suggestion for one matched misspelled name.
    pub(in crate::sema) fn rename_suggestion(
        &self,
        anchor: &DiagnosticAnchor,
        best: &NameMatch<String>,
    ) -> Option<DiagnosticSuggestion> {
        rename_suggestion(anchor, best)
    }

    /// Collect named lexical bindings visible from one scope cursor.
    fn collect_reference_names(
        &self,
        bindings: &dir::BindingTable<'_>,
        mut scope: dir::LocalScope,
        candidates: &mut Vec<String>,
    ) {
        loop {
            let current = bindings.get_scope(scope);

            // collect names declared before the visible scope mark
            for (key, _) in current.named_symbols_up_to(scope.mark) {
                if let Some(candidate) = self.reference_key_text(&key) {
                    candidates.push(candidate);
                }
            }

            let Some(parent) = current.parent else {
                return;
            };

            scope = parent;
        }
    }

    /// Return source text for an ordinary reference key.
    fn reference_key_text(&self, key: &dir::StaticKey) -> Option<String> {
        match key {
            dir::StaticKey::Name(name) => Some(self.strings().get(*name).to_string()),
            dir::StaticKey::Index(_) => None,
        }
    }
}
