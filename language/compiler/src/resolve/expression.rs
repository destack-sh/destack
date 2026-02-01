use destack_dir::{DependencySource, Expression, LocalNodeId, NodeTree, SymbolTable};

use crate::resolve::cache::{ResolveExpressionCache, ResolvePathCacheKey};
use crate::{Compiler, ResolveResult};

use destack_workspace::{Module, ModuleDir, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve an Expression.
    pub(super) fn resolve_expression(
        &self,
        module: &Module,
        dir: &ModuleDir,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        cache: &mut ResolveExpressionCache,
    ) -> ResolveResult<()> {
        let scope = symbols.get_scope(expression_id, tree);
        let scope = if module.language_type.is_declaration() {
            (scope.0, scope.1, destack_dir::LocalScopeMark::end())
        } else {
            scope
        };
        let expression = tree.get(expression_id);

        let expression: Expression = match expression {
            Expression::UnresolvedImport {
                source,
                kind,
                target,
                items,
                arguments,
            } => {
                let loader_override = self.loader_from_import_attributes(arguments.as_ref(), tree);
                let Some(remote_target) = self.resolve_import_maybe(
                    module,
                    dir,
                    profile,
                    expression_id.into_global_any(module.id),
                    *source,
                    *target,
                    loader_override,
                )?
                else {
                    return Ok(());
                };
                Expression::Import {
                    source: *source,
                    kind: *kind,
                    target: *target,
                    target_module: remote_target,
                    items: items.clone(),
                    arguments: arguments.clone(),
                }
            }

            Expression::UnresolvedReExport {
                target,
                kind,
                items,
            } => {
                let Some(remote_target) = self.resolve_import_maybe(
                    module,
                    dir,
                    profile,
                    expression_id.into_global_any(module.id),
                    DependencySource::ExportStatement,
                    *target,
                    None,
                )?
                else {
                    return Ok(());
                };
                Expression::ReExport {
                    target: *target,
                    target_module: remote_target,
                    kind: *kind,
                    items: items.clone(),
                }
            }

            Expression::UnresolvedPath {
                path,
                static_arguments,
                space_order,
            } => {
                let path = path.clone(); // (clone to release borrow on tree)
                let static_arguments = static_arguments.clone();
                if static_arguments.is_none() && path.segments.len() == 1 {
                    let name = path.first_segment().expect("path is empty");
                    let module_binding_scope_id =
                        self.module_binding_scope_for_global_expression(tree, expression_id);
                    let cache_key = ResolvePathCacheKey {
                        module_id: module.id,
                        scope_id: scope.0,
                        scope_mark: scope.2,
                        module_binding_scope_id,
                        name,
                        space_order: *space_order,
                    };
                    if let Some(cached) = cache.path_root(cache_key) {
                        cached
                    } else {
                        let resolved = self.resolve_absolute_path(
                            module,
                            expression_id,
                            expression_id.into_global_any(module.id),
                            profile,
                            scope,
                            &path,
                            static_arguments,
                            *space_order,
                            symbols,
                            tree,
                            cache,
                        )?;
                        cache.insert_path_root(cache_key, resolved.clone());
                        resolved
                    }
                } else {
                    self.resolve_absolute_path(
                        module,
                        expression_id,
                        expression_id.into_global_any(module.id),
                        profile,
                        scope,
                        &path,
                        static_arguments,
                        *space_order,
                        symbols,
                        tree,
                        cache,
                    )?
                }
            }

            Expression::UnresolvedBreak { target, value } => {
                let symbol_id = self.resolve_label_symbol(
                    module,
                    profile,
                    expression_id.into_global_any(module.id),
                    scope,
                    *target,
                    symbols,
                )?;
                Expression::Break {
                    target: Some(*target),
                    target_symbol: Some(symbol_id.into_global(module.id)),
                    value: *value,
                }
            }

            Expression::UnresolvedContinue { target } => {
                let symbol_id = self.resolve_label_symbol(
                    module,
                    profile,
                    expression_id.into_global_any(module.id),
                    scope,
                    *target,
                    symbols,
                )?;
                Expression::Continue {
                    target: Some(*target),
                    target_symbol: Some(symbol_id.into_global(module.id)),
                }
            }

            _ => return Ok(()),
        };
        *tree.get_mut(expression_id) = expression;

        Ok(())
    }
}
