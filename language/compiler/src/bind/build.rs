use destack_dir::{Expression, GlobalNodeIdAny, LocalNodeId, LocalSymbolId, StaticKey};

use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Bind the AST root expressions for a module.
    pub(super) fn bind_module_roots(&self, module: &mut Module) {
        let mut tree = module.dir.tree.write();
        let mut symbols = module.dir.symbols.write();
        let mut types = module.dir.types.write();
        let roots: Vec<LocalNodeId<Expression>> = module
            .ast
            .roots
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    (
                        module.dir.namespace_scope,
                        symbols.get_scope_mark(module.dir.namespace_scope),
                    ),
                    *expression,
                    None,
                    &mut tree,
                    &mut symbols,
                    &mut types,
                )
            })
            .collect();
        module.dir.roots.extend(roots);
    }

    /// Bind module exports and resolve conflicts.
    pub(super) fn bind_module_exports(&self, module: &mut Module) {
        let symbols = module.dir.symbols.read();

        // collect exported symbols
        let root_scope = symbols.get_scope_by_id(module.dir.namespace_scope);
        let exported_symbols: Vec<(GlobalNodeIdAny, LocalSymbolId)> = root_scope
            .named_symbols
            .iter()
            .filter_map(|(_, symbol_id)| {
                let symbol = symbols.get_symbol(*symbol_id);
                if let Some(primary_declaration) = symbol.primary_declaration
                    && symbol.export.is_some()
                {
                    Some((primary_declaration, *symbol_id))
                } else {
                    None
                }
            })
            .collect();

        // resolve exported symbols and check for conflicts
        let mut exported = module.dir.exported_symbols.write();
        for (_, symbol_id) in exported_symbols.iter() {
            let symbol = symbols.get_symbol(*symbol_id);
            let space = symbol.space;
            let Some(key) = symbol.name() else {
                continue; // should have a name but fine
            };
            let key = StaticKey::Name(key);
            // override if already exported (we error conflicting exports in a separate check)
            exported.insert((space, key), *symbol_id);
        }
    }
}

