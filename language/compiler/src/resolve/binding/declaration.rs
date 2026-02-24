use destack_dir::{
    Declaration, DependencyKind, Expression, GlobalSymbolId, ImportAliasTarget, LocalNodeId,
    NodeTree, NodeVisitor, NodeVisitorOptions, SymbolTable, Type, TypeKind, TypeTable,
    walk_expression,
};

use destack_workspace::{Module, ModuleDir, ProfileId};

use crate::resolve::binding::cache::ResolveExpressionCache;
use crate::{Compiler, ResolveResult};

impl Compiler {
    /// Resolve a Declaration node (updates target_symbol if applicable).
    pub(crate) fn resolve_declaration(
        &self,
        module: &Module,
        dir: &ModuleDir,
        profile: ProfileId,
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
                let target_type = *target_type;
                let target_symbol = *target_symbol;

                // bail if already resolved
                if target_symbol.is_some() {
                    return Ok(());
                }

                // resolve unresolved path nodes inside the target type
                // (yes this looks like ahack, but avoids resolving things we don't need to too early;
                //  like if we did this centrally and earlier in resolve/expression or something.)
                let unresolved_expression_ids = {
                    let mut collector = UnresolvedExpressionCollector::default();
                    let expression = tree.get(target_type);
                    collector.visit_expression(tree, target_type, expression);
                    collector.unresolved_expression_ids
                };
                let mut cache = ResolveExpressionCache::default();
                for expression_id in unresolved_expression_ids {
                    self.resolve_expression(
                        module,
                        dir,
                        profile,
                        expression_id,
                        tree,
                        symbols,
                        &mut cache,
                    )?;
                }

                // resolve the target symbol for the extension target
                let types = dir.types.read();
                let resolved_target_symbol =
                    self.target_symbol_for_type_expression(tree, &types, target_type);
                if let Some(resolved_target_symbol) = resolved_target_symbol
                    && let Declaration::Extension { target_symbol, .. } =
                        tree.get_mut(declaration_id)
                {
                    *target_symbol = Some(resolved_target_symbol);
                }

                Ok(())
            }

            Declaration::Type {
                descriptor,
                kind,
                value,
                ..
            } => {
                let symbol_id = descriptor.symbol;
                let symbol = symbols.get_symbol(symbol_id);

                // bail if symbol already has a target_symbol
                if symbol.target_symbol.is_some() {
                    return Ok(());
                }

                // only structural aliases should inherit target symbols
                if *kind != TypeKind::Structural {
                    return Ok(());
                }

                // extract the aliased type expression
                let value_expr = tree.get(*value);

                // extract the target symbol from the resolved reference expressions
                if let Some(resolved_target_symbol) = value_expr.target_symbol() {
                    // set the type alias symbol's target_symbol
                    symbols
                        .get_symbol_mut(symbol_id)
                        .resolve_to(resolved_target_symbol);
                }

                Ok(())
            }

            Declaration::ImportAlias {
                descriptor,
                kind,
                target,
            } => {
                let symbol_id = descriptor.symbol;
                let symbol = symbols.get_symbol(symbol_id);

                // bail if symbol already has a target_symbol
                if symbol.target_symbol.is_some() {
                    return Ok(());
                }

                // type-only aliases should only resolve type targets
                if *kind == DependencyKind::Type {
                    if let ImportAliasTarget::Path { value } = target
                        && let Some(resolved_target_symbol) = tree.get(*value).target_symbol()
                    {
                        symbols
                            .get_symbol_mut(symbol_id)
                            .resolve_to(resolved_target_symbol);
                    }
                    return Ok(());
                }

                // resolve path aliases to their target symbols
                if let ImportAliasTarget::Path { value } = target
                    && let Some(resolved_target_symbol) = tree.get(*value).target_symbol()
                {
                    symbols
                        .get_symbol_mut(symbol_id)
                        .resolve_to(resolved_target_symbol);
                }

                Ok(())
            }

            _ => Ok(()),
        }
    }

    /// Resolve a symbol reference from a type expression when possible.
    fn target_symbol_for_type_expression(
        &self,
        tree: &NodeTree,
        types: &TypeTable,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        // TODO #Cleanup: move type target resolution into a shared dir helper
        // unwrap parenthesized type references
        let mut target_id = expression_id;
        loop {
            let Expression::Parenthesized { expression } = tree.get(target_id) else {
                break;
            };
            target_id = *expression;
        }

        // allow direct references
        let expression = tree.get(target_id);
        if let Some(symbol) = expression.target_symbol() {
            return Some(symbol);
        }

        // allow explicit type expressions
        if let Expression::Type { value } = expression
            && let Type::Reference { symbol, .. } = types.get_type(*value)
        {
            return Some(*symbol);
        }

        // allow instantiation targets (like `Result<T, E>`)
        if let Expression::Instantiation { left, .. } = expression {
            return tree.get(*left).target_symbol();
        }

        None
    }
}

/// Collect unresolved path expressions within a subtree.
#[derive(Default)]
struct UnresolvedExpressionCollector {
    /// Visitor options.
    options: NodeVisitorOptions,
    /// Unresolved path expressions in the subtree.
    unresolved_expression_ids: Vec<LocalNodeId<Expression>>,
}

impl NodeVisitor for UnresolvedExpressionCollector {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // collect unresolved paths for targeted resolution
        if matches!(expression, Expression::UnresolvedPath { .. }) {
            self.unresolved_expression_ids.push(id);
        }

        walk_expression(self, tree, id, expression);
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
        test.compile_check_clean();

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
    bar() {}
}
"#,
        );
        test.resolve_module(module_id);
        test.compile_check_clean();

        // Foo
        let foo_symbol_id = test.resolve_to_symbol("test.ds", "Foo").unwrap();

        // extension Foo -> struct Foo
        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let tree = module.dir(profile).tree.read();
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
