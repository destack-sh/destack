use destack_core::closest_string;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckState;
use crate::{CompilerResult, diagnostic_suggestion_distance};

impl CheckState<'_> {
    /// Return the visible member key closest to one missing key.
    pub(in crate::check) fn closest_member_key(
        &mut self,
        receiver: dir::GlobalTypeId,
        key: &str,
    ) -> CompilerResult<Option<String>> {
        let keys = self.visible_member_keys(receiver)?;

        Ok(closest_string(
            key,
            keys,
            diagnostic_suggestion_distance(key),
        ))
    }

    /// Return a human readable path label.
    pub(in crate::check) fn path_label(&self, path: &dir::Path) -> String {
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

    /// Return the closest visible name for one unresolved single-segment path.
    pub(in crate::check) fn closest_reference_name(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> Option<String> {
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
            .global_target_by_key
            .keys()
        {
            if let Some(candidate) = self.reference_key_text(key) {
                candidates.push(candidate);
            }
        }

        closest_string(&name, candidates, diagnostic_suggestion_distance(&name))
    }

    /// Collect the member keys visible on one receiver.
    fn visible_member_keys(&mut self, receiver: dir::GlobalTypeId) -> CompilerResult<Vec<String>> {
        let mut current = self.settled_root(receiver)?;
        while let dir::Type::Form(form) = self.ty(current)? {
            current = self.settled_root(form.value)?;
        }

        let mut keys = Vec::new();
        match self.ty(current)? {
            dir::Type::Shape(shape) => {
                for field in self.shape_fields(current.module_id, shape.fields)? {
                    keys.push(self.format_static_key(&field.key));
                }
            }
            dir::Type::Reference(reference) => {
                if let Some(definition) = self.loaded_definition(reference.symbol) {
                    for member in definition.members() {
                        if member.space() == dir::MemberSpace::Static
                            && let Some(key) = member.key()
                        {
                            keys.push(self.format_static_key(&key));
                        }
                    }
                }
            }
            dir::Type::Instance(instance) => {
                if let Some(definition) = self.loaded_definition(instance.symbol) {
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
            for (key, symbol) in current.named_symbols_up_to(scope.mark) {
                let kind = bindings.get_symbol(symbol).kind;
                if !kind.is_visible_in(dir::SymbolSpace::Declaration) {
                    continue;
                }

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
            dir::StaticKey::Index(_) | dir::StaticKey::Symbol(_) => None,
        }
    }
}
