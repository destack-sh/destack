use std::sync::Arc;

use destack_dir::{DependencySource, Expression, LocalNodeId, NodeTree, NodeType, UnaryOperator};

use crate::resolve::binding::cache::{ResolveExpressionCache, ResolvePathCacheKey};
use crate::resolve::dependency::loader::LoaderAttribute;
use crate::{Compiler, ResolveError, ResolveResult};
use destack_workspace::{Module, ModuleDir, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return true when an unresolved identifier path is the direct operand of runtime `typeof`.
    fn unresolved_path_is_runtime_typeof_operand(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) -> bool {
        // only bare identifiers can remain unresolved under runtime typeof
        let Expression::UnresolvedPath {
            path,
            static_arguments: None,
            ..
        } = expression
        else {
            return false;
        };
        if path.segments.len() != 1 {
            return false;
        }

        // walk through parenthesized wrappers until the immediate unary parent
        let mut current_id = expression_id;
        loop {
            let Some(parent_id) = tree.get_parent(current_id.id) else {
                return false;
            };
            if parent_id.ty != NodeType::Expression {
                return false;
            }

            let parent_expression = tree.get(parent_id.into_typed::<Expression>());
            // allow optional parenthesized wrappers around the operand
            if let Expression::Parenthesized { expression } = parent_expression
                && *expression == current_id
            {
                current_id = parent_id.into_typed::<Expression>();
                continue;
            }

            // require runtime unary typeof as the direct parent
            if let Expression::Unary { operator, right } = parent_expression {
                return *operator == UnaryOperator::Typeof && *right == current_id;
            }

            return false;
        }
    }

    /// Resolve an Expression.
    pub(crate) fn resolve_expression(
        &self,
        module: &Module,
        dir: &mut ModuleDir,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        cache: &mut ResolveExpressionCache,
    ) -> ResolveResult<()> {
        let tree = dir.tree.as_ref();
        let scope = {
            let scope = dir.symbols.get_scope(expression_id, tree);
            let scope = (scope.0, scope.1.clone(), scope.2);
            if module.language_type.is_declaration() {
                (scope.0, scope.1, destack_dir::LocalScopeMark::end())
            } else {
                scope
            }
        };
        let expression = tree.get(expression_id).clone();
        let module_handle = self.program.modules.get(module.id);
        let module_handle = module_handle.as_ref();

        let expression: Expression = match expression {
            Expression::UnresolvedImport {
                source,
                kind,
                ref target,
                ref items,
                ref arguments,
            } => {
                if let destack_dir::ImportTarget::String(target) = target {
                    let loader_override =
                        match self.loader_from_import_attributes(arguments.as_ref(), tree) {
                            LoaderAttribute::None => None,
                            LoaderAttribute::Loader(loader) => Some(loader),
                            LoaderAttribute::InvalidType { value } => {
                                return Err(ResolveError::InvalidImportAttributeType {
                                    node: expression_id
                                        .into_global_any(module.id)
                                        .into_anchored(Some(profile)),
                                    value,
                                });
                            }
                        };
                    let Some(remote_target) = self.resolve_import_maybe(
                        &module_handle,
                        dir,
                        profile,
                        expression_id.into_global_any(module.id),
                        source,
                        *target,
                        kind,
                        loader_override,
                    )?
                    else {
                        return Ok(());
                    };
                    Expression::Import {
                        source,
                        kind,
                        target: target.clone(),
                        target_module: remote_target,
                        items: items.clone(),
                        arguments: arguments.clone(),
                    }
                } else {
                    expression
                }
            }

            Expression::UnresolvedReExport {
                ref target,
                kind,
                ref items,
                ref arguments,
            } => {
                let loader_override =
                    match self.loader_from_import_attributes(arguments.as_ref(), tree) {
                        LoaderAttribute::None => None,
                        LoaderAttribute::Loader(loader) => Some(loader),
                        LoaderAttribute::InvalidType { value } => {
                            return Err(ResolveError::InvalidImportAttributeType {
                                node: expression_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                                value,
                            });
                        }
                    };
                let Some(remote_target) = self.resolve_import_maybe(
                    &module_handle,
                    dir,
                    profile,
                    expression_id.into_global_any(module.id),
                    DependencySource::ExportStatement,
                    *target,
                    kind,
                    loader_override,
                )?
                else {
                    return Ok(());
                };
                Expression::ReExport {
                    target: *target,
                    target_module: remote_target,
                    kind,
                    items: items.clone(),
                    arguments: arguments.clone(),
                }
            }

            Expression::UnresolvedPath {
                ref path,
                ref static_arguments,
                space_order,
            } => {
                let exported_symbols = Arc::clone(&dir.exported_symbols);
                let symbols = Arc::clone(&dir.symbols);

                // track runtime typeof tolerance for missing symbols only
                let is_runtime_typeof_operand = self.unresolved_path_is_runtime_typeof_operand(
                    tree,
                    expression_id,
                    &expression,
                );

                // clone path data to release immutable tree borrows before resolution
                let path = path.clone();
                let static_arguments = static_arguments.clone();
                let resolve_result = if static_arguments.is_none() && path.segments.len() == 1 {
                    let name = path.first_segment().expect("path is empty");
                    let module_binding_scope_id =
                        self.module_binding_scope_for_global_expression(tree, expression_id);
                    let cache_key = ResolvePathCacheKey {
                        module_id: module.id,
                        scope_id: scope.0,
                        scope_mark: scope.2,
                        module_binding_scope_id,
                        name,
                        space_order,
                    };

                    // prefer cached scope root resolution for single segment paths
                    if let Some(cached) = cache.path_root(cache_key) {
                        Ok(cached)
                    } else {
                        let resolved = self.resolve_absolute_path(
                            module,
                            dir.namespace_symbol,
                            dir.namespace_scope,
                            dir.global_augmentation_scope,
                            exported_symbols.as_ref(),
                            expression_id,
                            expression_id.into_global_any(module.id),
                            profile,
                            (scope.0, &scope.1, scope.2),
                            &path,
                            static_arguments.clone(),
                            space_order,
                            symbols.as_ref(),
                            dir.tree_mut(),
                            cache,
                        );

                        // cache successful path root resolutions
                        if let Ok(expression) = &resolved {
                            cache.insert_path_root(cache_key, expression.clone());
                        }

                        resolved
                    }
                } else {
                    self.resolve_absolute_path(
                        module,
                        dir.namespace_symbol,
                        dir.namespace_scope,
                        dir.global_augmentation_scope,
                        exported_symbols.as_ref(),
                        expression_id,
                        expression_id.into_global_any(module.id),
                        profile,
                        (scope.0, &scope.1, scope.2),
                        &path,
                        static_arguments.clone(),
                        space_order,
                        symbols.as_ref(),
                        dir.tree_mut(),
                        cache,
                    )
                };

                // keep unresolved identifiers only for runtime typeof missing symbol probes
                match resolve_result {
                    Ok(resolved_expression) => resolved_expression,
                    Err(ResolveError::MissingSymbol { .. }) if is_runtime_typeof_operand => {
                        return Ok(());
                    }
                    Err(error) => return Err(error),
                }
            }

            Expression::UnresolvedBreak { target, value } => {
                let symbol_id = self.resolve_label_symbol(
                    module,
                    profile,
                    expression_id.into_global_any(module.id),
                    (scope.0, &scope.1, scope.2),
                    target,
                    &dir.symbols,
                )?;
                Expression::Break {
                    target: Some(target),
                    target_symbol: Some(symbol_id.into_global(module.id)),
                    value,
                }
            }

            Expression::UnresolvedContinue { target } => {
                let symbol_id = self.resolve_label_symbol(
                    module,
                    profile,
                    expression_id.into_global_any(module.id),
                    (scope.0, &scope.1, scope.2),
                    target,
                    &dir.symbols,
                )?;
                Expression::Continue {
                    target: Some(target),
                    target_symbol: Some(symbol_id.into_global(module.id)),
                }
            }

            _ => return Ok(()),
        };
        *dir.tree_mut().get_mut(expression_id) = expression;

        Ok(())
    }
}
