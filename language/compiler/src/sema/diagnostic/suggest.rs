use destack_core::{NameMatch, find_best_match};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::CheckState;
use crate::{CompilerResult, DiagnosticAnchor, diagnostic_suggestion_distance, rename_suggestion};
use destack_source::DiagnosticSuggestion;

impl CheckState<'_> {
    /// Return the visible member key closest to one missing key.
    pub(in crate::sema) fn closest_member_key(
        &mut self,
        receiver: dir::GlobalTypeId,
        key: &str,
    ) -> CompilerResult<Option<NameMatch<String>>> {
        let keys = self.visible_member_keys(receiver)?;

        Ok(find_best_match(
            key,
            keys,
            diagnostic_suggestion_distance(key),
        ))
    }

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

    /// Return one imported module declaring an unresolved name.
    pub(in crate::sema) fn declaring_sibling_module(
        &self,
        module: ModuleId,
        path: &dir::Path,
    ) -> Option<ModuleId> {
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

        // scan each imported module scope for the exact name
        let declares = |bindings: &dir::BindingTable<'_>| {
            let scope = bindings.module_scope();
            bindings
                .get_scope(scope)
                .named_symbols_up_to(scope.mark)
                .any(|(key, _)| {
                    matches!(key, dir::StaticKey::Name(key) if self.strings().get(key) == name)
                })
        };
        for imported in imported {
            let declared = match self.module_maybe(imported) {
                Some(state) => declares(&state.binding_table()),
                None => match self.external_modules.get(&imported) {
                    Some(external) => declares(&external.bindings),
                    None => continue,
                },
            };
            if declared {
                return Some(imported);
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

    /// Collect the member keys visible on one receiver.
    fn visible_member_keys(&mut self, receiver: dir::GlobalTypeId) -> CompilerResult<Vec<String>> {
        let mut current = self.shallow_resolve(receiver)?;
        while let dir::Type::Form(form) = self.ty(current)? {
            current = self.shallow_resolve(form.value)?;
        }

        let mut keys = Vec::new();
        match self.ty(current)? {
            dir::Type::Object(shape) => {
                for field in self.shape_properties(current.module_id, shape.properties)? {
                    keys.push(self.format_static_key(&field.key));
                }
            }
            dir::Type::Reference(reference) => {
                if let Some(definition) = self.definition_maybe(reference.symbol) {
                    for member in definition.members() {
                        if member.space() == dir::MemberSpace::Static
                            && let Some(key) = member.key()
                        {
                            keys.push(self.format_static_key(&key));
                        }
                    }
                }
            }
            dir::Type::Application(instance) => {
                if let Some(definition) = self.definition_maybe(instance.symbol) {
                    for member in definition.members() {
                        if let Some(key) = member.key() {
                            keys.push(self.format_static_key(&key));
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(keys)
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
