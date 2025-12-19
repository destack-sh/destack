use destack_dir::{Declaration, Expression, GlobalSymbolId, LocalNodeId, NodeTree, SymbolTable};

use destack_workspace::{Module, ModuleDir};

use crate::{Compiler, ResolveResult};

impl Compiler {
    /// Resolve a Declaration node (updates target_symbol if applicable).
    pub(super) fn resolve_declaration(
        &self,
        _module: &Module,
        _dir: &ModuleDir,
        declaration_id: LocalNodeId<Declaration>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> ResolveResult<()> {
        let declaration = tree.get(declaration_id);
        match declaration {
            Declaration::Extension {
                target_type,
                target_symbol,
                ..
            } => {
                // bail if already resolved
                if target_symbol.is_some() {
                    return Ok(());
                }

                // resolve the target symbol
                let target_type_expr = tree.get(*target_type);
                let resolved_target_symbol = Self::extract_target_symbol(target_type_expr);
                if let Some(resolved_target_symbol) = resolved_target_symbol
                    && let Declaration::Extension { target_symbol, .. } =
                        tree.get_mut(declaration_id)
                {
                    *target_symbol = Some(resolved_target_symbol);
                }

                Ok(())
            }

            Declaration::Type {
                descriptor, value, ..
            } => {
                let symbol_id = descriptor.symbol;
                let symbol = symbols.get_symbol(symbol_id);

                // bail if symbol already has a target_symbol
                if symbol.target_symbol.is_some() {
                    return Ok(());
                }

                // extract the aliased type expression
                let value_expr = tree.get(*value);

                // extract the target symbol from the resolved reference expressions
                if let Some(resolved_target_symbol) = Self::extract_target_symbol(value_expr) {
                    // set the type alias symbol's target_symbol
                    symbols
                        .get_symbol_mut(symbol_id)
                        .resolve_to(resolved_target_symbol);
                }

                Ok(())
            }

            _ => Ok(()),
        }
    }

    /// Extract target_symbol from a resolved expression.
    fn extract_target_symbol(expr: &Expression) -> Option<GlobalSymbolId> {
        match expr {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::Declaration;

    use crate::TestProgram;

    /// Test that type alias symbols have their target_symbol set when aliasing a named type.
    #[test]
    fn test_resolve_type_alias_target_symbol() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {}
type Bar = Foo;
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        // Foo
        let foo_symbol_id = test.resolve_to_symbol("test.ds", "Foo").unwrap();

        // Bar = Foo
        let bar_symbol_id = test.resolve_to_symbol("test.ds", "Bar").unwrap();
        let bar_symbol = test.symbol_by_id(bar_symbol_id);

        // Bar -> Foo
        assert!(
            bar_symbol.target_symbol.is_some(),
            "Type alias symbol should have target_symbol set"
        );
        assert_eq!(
            bar_symbol.target_symbol.unwrap(),
            foo_symbol_id,
            "Type alias should point to Foo struct"
        );
    }

    /// Test that extension declarations have their target_symbol set after resolution.
    #[test]
    fn test_resolve_extension_target_symbol() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {}
extension for Foo {
    fn bar() {}
}
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        // Foo
        let foo_symbol_id = test.resolve_to_symbol("test.ds", "Foo").unwrap();

        // extension Foo -> struct Foo
        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let extensions: Vec<_> = tree
            .iter_node_ids_of_type::<Declaration>()
            .into_iter()
            .filter_map(|id| match tree.get(id) {
                Declaration::Extension { target_symbol, .. } => Some(*target_symbol),
                _ => None,
            })
            .collect();

        assert_eq!(extensions.len(), 1);
        let extension_target = extensions[0];
        assert!(
            extension_target.is_some(),
            "extension target_symbol should be set"
        );
        assert_eq!(
            extension_target.unwrap(),
            foo_symbol_id,
            "extension should point to Foo struct"
        );
    }
}
