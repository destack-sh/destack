use tspp_core::{NameMatch, find_best_match};
use tspp_dir as dir;
use tspp_source::{DiagnosticSuggestion, ModuleId};

use crate::export::ExportLookup;
use crate::sema::CheckState;
use crate::{CompilerResult, DiagnosticAnchor, diagnostic_suggestion_distance, rename_suggestion};

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

    /// Find one import candidate from modules named by explicit imports.
    pub(in crate::sema) fn find_import_candidate(
        &mut self,
        module: ModuleId,
        path: &dir::Path,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let [name] = path.segments.as_slice() else {
            return Ok(None);
        };

        let key = dir::ExportKey::named(dir::StaticKey::Name(*name));

        // search exports from explicitly imported modules
        let resolved = self.module(module).resolved.clone();
        for resolution in resolved.imports.resolution_by_symbol.values() {
            for declaration in resolution.declarations() {
                let candidate_module = declaration.module();
                if candidate_module != module {
                    let lookup = self.exports.resolve_export_target(
                        self.artifacts,
                        candidate_module,
                        key,
                    )?;
                    if let ExportLookup::Found(resolution) = lookup
                        && let Some(symbol) = resolution.declaration.single_symbol()
                    {
                        return Ok(Some(symbol));
                    }
                }
            }
        }

        Ok(None)
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
