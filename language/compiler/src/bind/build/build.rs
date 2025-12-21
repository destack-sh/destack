use destack_dir::{Expression, GlobalNodeIdAny, LocalNodeId, LocalSymbolId, StaticKey};

use destack_workspace::{Module, ModuleAst};

use crate::Compiler;

impl Compiler {
    /// Bind the AST root expressions for a module.
    pub(super) fn bind_module_roots(
        &self,
        module: &Module,
        ast: &ModuleAst,
    ) -> Vec<LocalNodeId<Expression>> {
        let dir = module.dir();
        let mut tree = dir.tree.write();
        let mut symbols = dir.symbols.write();
        let mut types = dir.types.write();
        ast.roots
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    ast,
                    (
                        dir.namespace_scope,
                        symbols.get_scope_mark(dir.namespace_scope),
                    ),
                    *expression,
                    None,
                    &mut tree,
                    &mut symbols,
                    &mut types,
                )
            })
            .collect()
    }

    /// Bind module exports and resolve conflicts.
    pub(super) fn bind_module_exports(&self, module: &mut Module) {
        let dir = module.dir();
        let symbols = dir.symbols.read();

        // collect exported symbols
        let root_scope = symbols.get_scope_by_id(dir.namespace_scope);
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
        let mut exported = dir.exported_symbols.write();
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
