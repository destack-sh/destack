#![allow(unreachable_pub)]

use destack_dir::{
    Declaration, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeMark, LocalSymbolId, Node,
    Scope, StaticKey, SymbolSpace, SymbolTable,
};

use crate::TestProgram;

impl TestProgram {
    /// Resolve a symbol path in a module.
    pub fn resolve_to_symbol(&self, module_uri: &str, path: &str) -> Option<GlobalSymbolId> {
        let module = self.module(module_uri);
        let module = module.as_ref();
        let profile = self.default_profile_id(module.id);
        let dir = self.artifact_dir(module.id, profile);
        let symbols = &dir.symbols;

        let segments: Vec<&str> = path.split('.').collect();
        if segments.is_empty() {
            return None;
        }

        // resolve the first segment as an absolute symbol
        let first_segment = segments[0];
        let first_segment = self.program.strings.intern(first_segment);
        let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
        let symbol_id = self.resolve_absolute_symbol(
            symbols,
            (namespace_scope, LocalScopeMark::end()),
            StaticKey::Name(first_segment),
        )?;

        // resolve remaining segments as relative
        if segments.len() > 1 {
            let remaining_segments = &segments[1..];
            let symbol_id =
                self.resolve_relative_symbol(symbols, symbol_id.into_local(), remaining_segments)?;
            Some(symbol_id)
        } else {
            Some(symbol_id)
        }
    }

    /// Resolve a path to a primary declaration.
    pub fn resolve_to_declaration(
        &self,
        module_uri: &str,
        path: &str,
    ) -> Option<(GlobalSymbolId, GlobalNodeIdAny)> {
        let symbol_id = self.resolve_to_symbol(module_uri, path)?;
        let module = self.module(module_uri);
        let module = module.as_ref();
        let profile = self.default_profile_id(module.id);
        let dir = self.artifact_dir(module.id, profile);
        let symbols = &dir.symbols;
        let symbol = symbols.get_symbol(symbol_id.into_local());
        Some((symbol_id, symbol.primary_declaration?))
    }

    /// Resolve a path to a primary declaration's node.
    pub fn resolve_to_node<T: Node>(
        &self,
        module_uri: &str,
        path: &str,
    ) -> Option<(GlobalSymbolId, LocalNodeId<T>)> {
        let (symbol_id, declaration) = self.resolve_to_declaration(module_uri, path)?;
        Some((symbol_id, declaration.into_local_typed::<T>()))
    }

    /// Resolve a function symbol by simple name.
    pub fn function_symbol_by_name(&self, module_uri: &str, name: &str) -> Option<GlobalSymbolId> {
        // load module state
        let module = self.module(module_uri);
        let module = module.as_ref();
        let profile = self.default_profile_id(module.id);
        let dir = self.artifact_dir(module.id, profile);
        let tree = &dir.tree;

        // scan declarations for a matching function name
        let name_id = self.program.strings.intern(name);
        for (_, declaration) in tree.iter_nodes_of_type::<Declaration>() {
            let Declaration::Function(declaration) = declaration else {
                continue;
            };
            let Some(declaration_name) = declaration.name else {
                continue;
            };
            if declaration_name.string() == name_id {
                return Some(declaration.symbol.into_global(module.id));
            }
        }

        None
    }

    /// Resolve a declaration symbol by simple name.
    pub fn declaration_symbol_by_name(
        &self,
        module_uri: &str,
        name: &str,
    ) -> Option<GlobalSymbolId> {
        // declaration names resolve through the same top level symbol path lookup
        self.resolve_to_symbol(module_uri, name)
    }

    /// Resolve an absolute symbol by walking up scopes.
    fn resolve_absolute_symbol(
        &self,
        symbols: &SymbolTable,
        scope: (&Scope, LocalScopeMark),
        key: StaticKey,
    ) -> Option<GlobalSymbolId> {
        let mut scope = scope;
        loop {
            // find symbol in current scope
            if let Some(symbol_id) = symbols.find_active_symbol_up_to(scope.0, key, scope.1) {
                return Some(symbol_id.into_global(symbols.module_id));
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
    fn resolve_relative_symbol(
        &self,
        symbols: &SymbolTable,
        symbol_id: LocalSymbolId,
        path_segments: &[&str],
    ) -> Option<GlobalSymbolId> {
        let mut current_symbol_id = symbol_id;
        let mut symbol = symbols.get_symbol(current_symbol_id);
        let mut scope = symbols.get_scope_by_id(symbol.scope.0);

        // resolve path segments
        for segment in path_segments {
            let segment_id = self.program.strings.intern(segment);
            let key = StaticKey::Name(segment_id);
            if let Some(next_symbol_id) = symbols.find_active_symbol(scope, key) {
                current_symbol_id = next_symbol_id;
                symbol = symbols.get_symbol(current_symbol_id);
                scope = symbols.get_scope_by_id(symbol.scope.0);
            } else {
                return None;
            }
        }

        Some(current_symbol_id.into_global(symbols.module_id))
    }

    /// Resolve a label symbol by name.
    pub fn resolve_label_symbol(&self, module_uri: &str, name: &str) -> Option<GlobalSymbolId> {
        let module = self.module(module_uri);
        let module = module.as_ref();
        let profile = self.default_profile_id(module.id);
        let dir = self.artifact_dir(module.id, profile);
        let symbols = &dir.symbols;
        let name_id = self.program.strings.intern(name);

        // search all symbols for a matching label symbol
        for (idx, symbol) in symbols.symbols().enumerate() {
            if symbol.space == SymbolSpace::Label
                && let Some(StaticKey::Name(key_name)) = symbol.key
                && key_name == name_id
            {
                return Some(
                    LocalSymbolId::new_typed(idx as u32, symbol.ty).into_global(module.id),
                );
            }
        }

        None
    }
}
