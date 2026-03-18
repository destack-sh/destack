use crate::resolve::binding::cache::ResolveExpressionCache;
use crate::{Compiler, ResolveResult};
use destack_dir::{
    Declaration, DependencyKind, Expression, GlobalSymbolId, ImportAliasTarget, LocalNodeId,
    NodeTree, NodeVisitor, NodeVisitorOptions, SymbolTable, Type, TypeKind, TypeTable,
    walk_expression,
};
use destack_workspace::{ExportedSymbolTable, ImportedModuleTable, Module, ProfileId};

impl Compiler {
    /// Resolve a Declaration node (updates target_symbol if applicable).
    pub(crate) fn resolve_declaration(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &TypeTable,
        imported_modules: &mut ImportedModuleTable,
        namespace_symbol: destack_dir::LocalSymbolId,
        namespace_scope: destack_dir::LocalScopeId,
        global_augmentation_scope: destack_dir::LocalScopeId,
        exported_symbols: &mut ExportedSymbolTable,
        declaration_id: LocalNodeId<Declaration>,
    ) -> ResolveResult<()> {
        let declaration = tree.get(declaration_id).clone();
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

                // resolve unresolved path nodes inside the target type
                // (yes this looks like a hack, but avoids resolving things we don't need to too early;
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
                        profile,
                        tree,
                        symbols,
                        types,
                        imported_modules,
                        namespace_symbol,
                        namespace_scope,
                        global_augmentation_scope,
                        exported_symbols,
                        expression_id,
                        &mut cache,
                    )?;
                }

                // resolve the target symbol for the extension target
                let resolved_target_symbol =
                    self.target_symbol_for_type_expression(tree, types, target_type);
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
                if kind != TypeKind::Structural {
                    return Ok(());
                }

                // extract the aliased type expression
                let value_expr = tree.get(value);

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
                if kind == DependencyKind::Type {
                    if let ImportAliasTarget::Path { value } = target
                        && let Some(resolved_target_symbol) = tree.get(value).target_symbol()
                    {
                        symbols
                            .get_symbol_mut(symbol_id)
                            .resolve_to(resolved_target_symbol);
                    }
                    return Ok(());
                }

                // resolve path aliases to their target symbols
                if let ImportAliasTarget::Path { value } = target
                    && let Some(resolved_target_symbol) = tree.get(value).target_symbol()
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
